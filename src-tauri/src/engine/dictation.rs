//! Диктовка: захват микрофона (cpal) + «тёплый» ASR для минимальной задержки.
//!
//! `cpal::Stream` не Send (на macOS CoreAudio), поэтому запись живёт на отдельном
//! потоке, который владеет стримом; сюда данные приходят в общий буфер. Так же и ASR:
//! `OfflineRecognizer` держим на выделенном воркер-потоке (не гоняем между потоками),
//! общаемся через каналы. Модель загружается один раз и переиспользуется — на короткой
//! фразе это разница между «моментально» и «секунда на загрузку каждый раз».

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::engine::{asr::Asr, decode, models};

const SR: u32 = 16000;

// ───────────────────────── Захват микрофона ─────────────────────────

struct Recorder {
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    meta: Arc<Mutex<(u32, u16)>>, // (частота источника, число каналов)
    handle: thread::JoinHandle<()>,
}

fn slot() -> &'static Mutex<Option<Recorder>> {
    static R: OnceLock<Mutex<Option<Recorder>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(None))
}

/// Идёт ли сейчас запись.
pub fn is_recording() -> bool {
    slot().lock().unwrap().is_some()
}

/// Список устройств ввода (имена) — для выбора в настройках.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    match host.input_devices() {
        Ok(it) => it.filter_map(|d| d.name().ok()).collect(),
        Err(_) => Vec::new(),
    }
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

fn build_stream(
    device_name: &str,
    samples: Arc<Mutex<Vec<f32>>>,
    meta: Arc<Mutex<(u32, u16)>>,
) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = pick_device(&host, device_name).ok_or("микрофон не найден")?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("конфиг микрофона: {e}"))?;
    let src_rate = config.sample_rate().0;
    let channels = config.channels();
    *meta.lock().unwrap() = (src_rate, channels);

    let err_fn = |e| eprintln!("cpal input error: {e}");
    let fmt = config.sample_format();
    let cfg: cpal::StreamConfig = config.into();

    let stream = match fmt {
        cpal::SampleFormat::F32 => {
            let buf = samples.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[f32], _: &_| buf.lock().unwrap().extend_from_slice(data),
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let buf = samples.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[i16], _: &_| {
                    buf.lock()
                        .unwrap()
                        .extend(data.iter().map(|&s| s as f32 / 32768.0))
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let buf = samples.clone();
            device.build_input_stream(
                &cfg,
                move |data: &[u16], _: &_| {
                    buf.lock()
                        .unwrap()
                        .extend(data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0))
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

/// Начать запись с указанного устройства (пусто → устройство по умолчанию).
/// Повторный вызов во время записи — no-op.
pub fn start(device_name: &str) -> Result<(), String> {
    let mut guard = slot().lock().unwrap();
    if guard.is_some() {
        return Ok(());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let meta = Arc::new(Mutex::new((SR, 1u16)));
    let (tx, rx) = mpsc::channel::<Result<(), String>>();

    let (dev, s2, m2, stop2) = (
        device_name.to_string(),
        samples.clone(),
        meta.clone(),
        stop.clone(),
    );
    let handle = thread::spawn(move || match build_stream(&dev, s2, m2) {
        Ok(stream) => {
            if let Err(e) = stream.play() {
                let _ = tx.send(Err(format!("микрофон не запустился: {e}")));
                return;
            }
            let _ = tx.send(Ok(()));
            while !stop2.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(20));
            }
            drop(stream); // остановить и освободить устройство на этом же потоке
        }
        Err(e) => {
            let _ = tx.send(Err(e));
        }
    });

    match rx.recv() {
        Ok(Ok(())) => {
            *guard = Some(Recorder {
                stop,
                samples,
                meta,
                handle,
            });
            Ok(())
        }
        Ok(Err(e)) => {
            let _ = handle.join();
            Err(e)
        }
        Err(_) => Err("микрофон: поток не запустился".into()),
    }
}

/// Остановить запись и вернуть PCM 16 кГц mono.
pub fn stop() -> Result<Vec<f32>, String> {
    let rec = slot().lock().unwrap().take().ok_or("запись не активна")?;
    rec.stop.store(true, Ordering::Relaxed);
    let _ = rec.handle.join();
    let (src_rate, channels) = *rec.meta.lock().unwrap();
    let raw = std::mem::take(&mut *rec.samples.lock().unwrap());
    let mono = downmix(&raw, channels);
    Ok(decode::resample_linear(&mono, src_rate, SR))
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
