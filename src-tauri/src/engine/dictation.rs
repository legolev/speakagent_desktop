//! Диктовка: захват микрофона (cpal) + «тёплый» ASR для минимальной задержки.
//!
//! Чтобы не терять первое слово: поток микрофона держим ОТКРЫТЫМ между записями
//! (закрываем по простою), а входящее аудио пишем в кольцевой pre-roll буфер. Открытие/
//! старт устройства (особенно WASAPI на Windows) занимает десятки-сотни мс — если открывать
//! стрим только по нажатию хоткея, начало речи не успевает записаться. Тёплый стрим + pre-roll
//! эту задержку убирают: на «взводе» запись сразу подхватывает последние ~0.4 с.
//!
//! `cpal::Stream` не Send (CoreAudio/WASAPI), поэтому живёт на выделенном аудио-потоке,
//! которым управляем командами. ASR тоже на своём воркер-потоке (OfflineRecognizer не гоняем
//! между потоками) — модель грузится один раз и переиспользуется.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::engine::{asr::Asr, decode, models};

const SR: u32 = 16000;
const PREROLL_SEC: f32 = 0.4; // сколько аудио «до нажатия» подхватываем (тёплый стрим)
const IDLE_SECS: u64 = 60; // через сколько простоя закрыть микрофон

// ───────────────────────── Аудио-движок (тёплый стрим) ─────────────────────────

/// Общее состояние, делимое со звуковым callback.
#[derive(Default)]
struct AudioBuf {
    ring: VecDeque<f32>, // pre-roll (интерливед)
    rec: Vec<f32>,       // активная запись (интерливед)
    armed: bool,
    maxlen: usize, // ёмкость ring (семплы)
    src_rate: u32,
    channels: u16,
}

fn armed_flag() -> &'static AtomicBool {
    static A: OnceLock<AtomicBool> = OnceLock::new();
    A.get_or_init(|| AtomicBool::new(false))
}

/// Идёт ли сейчас запись (взведён ли триггер).
pub fn is_recording() -> bool {
    armed_flag().load(Ordering::Relaxed)
}

enum Cmd {
    Arm(String, Sender<Result<(), String>>),
    Disarm(Sender<Result<Vec<f32>, String>>),
}

fn engine() -> &'static Mutex<Option<Sender<Cmd>>> {
    static E: OnceLock<Mutex<Option<Sender<Cmd>>>> = OnceLock::new();
    E.get_or_init(|| Mutex::new(None))
}

fn engine_tx() -> Sender<Cmd> {
    let mut g = engine().lock().unwrap();
    if let Some(tx) = g.as_ref() {
        return tx.clone();
    }
    let (tx, rx) = mpsc::channel::<Cmd>();
    thread::spawn(move || audio_engine(rx));
    *g = Some(tx.clone());
    tx
}

fn audio_engine(rx: mpsc::Receiver<Cmd>) {
    let shared = Arc::new(Mutex::new(AudioBuf::default()));
    let mut stream: Option<cpal::Stream> = None; // живёт на этом потоке (!Send)
    let mut cur_device = String::new();
    let mut last_active = Instant::now();

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Cmd::Arm(device, reply)) => {
                if stream.is_none() || device != cur_device {
                    stream = None; // закрыть прежний
                    match open_stream(&device, shared.clone()) {
                        Ok(s) => {
                            if let Err(e) = s.play() {
                                let _ = reply.send(Err(format!("микрофон не запустился: {e}")));
                                continue;
                            }
                            stream = Some(s);
                            cur_device = device.clone();
                        }
                        Err(e) => {
                            let _ = reply.send(Err(e));
                            continue;
                        }
                    }
                }
                {
                    let mut sh = shared.lock().unwrap();
                    sh.rec = sh.ring.iter().copied().collect(); // pre-roll как затравка записи
                    sh.armed = true;
                }
                armed_flag().store(true, Ordering::Relaxed);
                last_active = Instant::now();
                let _ = reply.send(Ok(()));
            }
            Ok(Cmd::Disarm(reply)) => {
                armed_flag().store(false, Ordering::Relaxed);
                let (rec, sr, ch) = {
                    let mut sh = shared.lock().unwrap();
                    sh.armed = false;
                    (std::mem::take(&mut sh.rec), sh.src_rate, sh.channels)
                };
                let mono = downmix(&rec, ch);
                let pcm = decode::resample_linear(&mono, sr.max(1), SR);
                let _ = reply.send(Ok(pcm));
                last_active = Instant::now();
            }
            Err(RecvTimeoutError::Timeout) => {
                if stream.is_some()
                    && !armed_flag().load(Ordering::Relaxed)
                    && last_active.elapsed() > Duration::from_secs(IDLE_SECS)
                {
                    stream = None; // закрыть микрофон по простою
                    cur_device.clear();
                    shared.lock().unwrap().ring.clear();
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Кольцевой pre-roll + (если взведено) активная запись. Держим лок коротко.
fn feed(shared: &Mutex<AudioBuf>, samples: impl Iterator<Item = f32>) {
    let mut sh = shared.lock().unwrap();
    let maxlen = sh.maxlen;
    for s in samples {
        sh.ring.push_back(s);
        if maxlen > 0 && sh.ring.len() > maxlen {
            sh.ring.pop_front();
        }
        if sh.armed {
            sh.rec.push(s);
        }
    }
}

fn open_stream(device_name: &str, shared: Arc<Mutex<AudioBuf>>) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = pick_device(&host, device_name).ok_or("микрофон не найден")?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("конфиг микрофона: {e}"))?;
    let src_rate = config.sample_rate().0;
    let channels = config.channels();
    {
        let mut sh = shared.lock().unwrap();
        sh.src_rate = src_rate;
        sh.channels = channels;
        sh.maxlen = (PREROLL_SEC * src_rate as f32 * channels as f32) as usize;
        sh.ring.clear();
        sh.rec.clear();
    }

    let err_fn = |e| eprintln!("cpal input error: {e}");
    let fmt = config.sample_format();
    let cfg: cpal::StreamConfig = config.into();
    let stream = match fmt {
        cpal::SampleFormat::F32 => {
            let sh = shared.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[f32], _: &_| feed(&sh, data.iter().copied()),
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let sh = shared.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[i16], _: &_| feed(&sh, data.iter().map(|&s| s as f32 / 32768.0)),
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let sh = shared.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[u16], _: &_| {
                    feed(&sh, data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0))
                },
                err_fn,
                None,
            )
        }
        other => return Err(format!("неподдерживаемый формат микрофона: {other:?}")),
    }
    .map_err(|e| format!("не удалось открыть микрофон: {e}"))?;
    Ok(stream)
}

fn pick_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    if !name.is_empty() {
        if let Ok(mut it) = host.input_devices() {
            if let Some(d) = it.find(|d| d.name().map(|n| n == name).unwrap_or(false)) {
                return Some(d);
            }
        }
    }
    host.default_input_device()
}

fn downmix(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let ch = channels as usize;
    interleaved
        .chunks(ch)
        .map(|f| f.iter().sum::<f32>() / ch as f32)
        .collect()
}

/// Список устройств ввода (имена) — для выбора в настройках.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    match host.input_devices() {
        Ok(it) => it.filter_map(|d| d.name().ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/// Начать запись (взвести триггер). На тёплом стриме — мгновенно, с pre-roll.
pub fn start(device_name: &str) -> Result<(), String> {
    let tx = engine_tx();
    let (rtx, rrx) = mpsc::channel();
    tx.send(Cmd::Arm(device_name.to_string(), rtx))
        .map_err(|_| "аудио-движок недоступен".to_string())?;
    rrx.recv().map_err(|_| "аудио-движок не ответил".to_string())?
}

/// Остановить запись и вернуть PCM 16 кГц mono (стрим остаётся тёплым).
pub fn stop() -> Result<Vec<f32>, String> {
    let tx = engine_tx();
    let (rtx, rrx) = mpsc::channel();
    tx.send(Cmd::Disarm(rtx))
        .map_err(|_| "аудио-движок недоступен".to_string())?;
    rrx.recv().map_err(|_| "аудио-движок не ответил".to_string())?
}

// ───────────────────────── «Тёплый» ASR ─────────────────────────

enum Job {
    Transcribe(Vec<f32>, Sender<Result<String, String>>),
}

fn worker() -> &'static Mutex<Option<Sender<Job>>> {
    static W: OnceLock<Mutex<Option<Sender<Job>>>> = OnceLock::new();
    W.get_or_init(|| Mutex::new(None))
}

fn num_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| (n.get() as i32).clamp(1, 16))
        .unwrap_or(4)
}

fn ensure_worker() -> Sender<Job> {
    let mut g = worker().lock().unwrap();
    if let Some(tx) = g.as_ref() {
        return tx.clone();
    }
    let (tx, rx) = mpsc::channel::<Job>();
    thread::spawn(move || {
        // (id модели, загруженный распознаватель) — держим «тёплым» между вызовами.
        let mut warm: Option<(String, Asr)> = None;
        while let Ok(job) = rx.recv() {
            match job {
                Job::Transcribe(pcm, reply) => {
                    let want = models::dict_asr_id();
                    let reload = warm.as_ref().map(|(id, _)| id != &want).unwrap_or(true);
                    if reload {
                        let loaded = models::dict_asr_files()
                            .ok_or_else(|| {
                                "Модель распознавания для диктовки не установлена. Откройте «Настройки»."
                                    .to_string()
                            })
                            .and_then(|f| Asr::load(&f, num_threads()));
                        match loaded {
                            Ok(asr) => warm = Some((want.clone(), asr)),
                            Err(e) => {
                                let _ = reply.send(Err(e));
                                continue;
                            }
                        }
                    }
                    let (_, asr) = warm.as_ref().unwrap();
                    let cancel = AtomicBool::new(false);
                    let vad = models::vad();
                    let text = asr.transcribe(&pcm, vad.as_deref(), &cancel, |_, _, _| {});
                    let _ = reply.send(Ok(text));
                }
            }
        }
    });
    *g = Some(tx.clone());
    tx
}

/// Распознать PCM 16 кГц mono на «тёплом» движке диктовки. Блокирующий вызов —
/// запускать из фонового потока.
pub fn transcribe(pcm: Vec<f32>) -> Result<String, String> {
    let tx = ensure_worker();
    let (rtx, rrx) = mpsc::channel();
    tx.send(Job::Transcribe(pcm, rtx))
        .map_err(|_| "движок диктовки недоступен".to_string())?;
    rrx.recv()
        .map_err(|_| "движок диктовки не ответил".to_string())?
}
