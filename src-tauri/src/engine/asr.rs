//! ASR через sherpa-onnx. Поддержка GigaAM (CTC), Parakeet (transducer), Whisper.
//! Нарезка длинного аудио — по VAD (Silero), чтобы границы падали в тишину, а не
//! посреди слова (иначе «мусор» на стыках). Если модель VAD недоступна — слепые окна.
//!
//! Конвейер: VAD-нарезчик (свой поток) → очередь сегментов → N декодеров параллельно →
//! сборщик восстанавливает порядок и отдаёт прогресс. Для файлов N = ядра, а у каждого
//! декодера ОДИН поток ORT: мелкие операторы модели плохо масштабируются внутри одного
//! прогона (барьеры синхронизации), а независимые сегменты — почти линейно. Замер на
//! 4 ядрах (GigaAM v3, 12 мин): 1×4 потока — 31 с, 4×1 — 21 с, текст байт-в-байт тот же.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};

use sherpa_onnx::{
    OfflineNemoEncDecCtcModelConfig, OfflineRecognizer, OfflineRecognizerConfig,
    OfflineTransducerModelConfig, OfflineWhisperModelConfig, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};

const SR: i32 = 16000;
const CHUNK_SEC: usize = 20; // слепой фолбэк, если нет VAD
const VAD_WINDOW: usize = 512; // окно Silero VAD (samples)
const VAD_MAX_SPEECH_SEC: f32 = 20.0; // не даём offline-CTC слишком длинный кусок
const MAX_TRANSDUCER_WORKERS: usize = 8; // Parakeet крупнее GigaAM — не раздуваем память

/// Тип движка модели.
#[derive(Clone, Copy, PartialEq)]
pub enum Engine {
    NemoCtc,    // GigaAM
    Transducer, // Parakeet (nemo_transducer)
    Whisper,    // Whisper
}

/// Пути к файлам активной модели + её тип.
pub struct AsrFiles {
    pub engine: Engine,
    pub model: String,   // ctc: model; transducer/whisper: encoder
    pub decoder: String, // transducer/whisper: decoder; ctc: ""
    pub joiner: String,  // transducer: joiner; иначе ""
    pub tokens: String,
    pub language: String, // whisper: язык ("ru"); иначе ""
}

/// Слово с временем начала и конца (сек от начала файла) — для привязки к спикерам.
pub struct Word {
    pub text: String,
    pub start: f32,
    pub end: f32,
}

/// Речевой кусок для декодера: срез исходного буфера (без копирования).
struct Seg {
    idx: usize,
    start: usize,
    len: usize,
}

/// Распознаватель с загруженной моделью. Создаётся один раз, переиспользуется.
pub struct Asr {
    rec: OfflineRecognizer,
    /// Сколько сегментов декодируем одновременно (1 — последовательно).
    workers: usize,
}

impl Asr {
    /// Интерактивный режим (диктовка): один декодер с `num_threads` потоками ORT —
    /// минимальная задержка на одном коротком куске.
    pub fn load(f: &AsrFiles, num_threads: i32) -> Result<Self, String> {
        Self::build(f, num_threads, 1)
    }

    /// Пакетный режим (файлы): независимые сегменты декодируются параллельно, по одному
    /// потоку ORT на декодер. Whisper — последовательно: его декодер sherpa на каждом
    /// вызове перезаписывает общий конфиг (гонка при параллельных вызовах).
    pub fn load_parallel(f: &AsrFiles) -> Result<Self, String> {
        let workers = match f.engine {
            Engine::NemoCtc => crate::engine::hw::asr_workers(),
            Engine::Transducer => crate::engine::hw::asr_workers().min(MAX_TRANSDUCER_WORKERS),
            Engine::Whisper => 1,
        };
        if workers <= 1 {
            Self::build(f, crate::engine::hw::ort_threads(), 1)
        } else {
            Self::build(f, 1, workers)
        }
    }

    fn build(f: &AsrFiles, num_threads: i32, workers: usize) -> Result<Self, String> {
        let mut config = OfflineRecognizerConfig::default();
        match f.engine {
            Engine::NemoCtc => {
                config.model_config.nemo_ctc = OfflineNemoEncDecCtcModelConfig {
                    model: Some(f.model.clone()),
                };
            }
            Engine::Transducer => {
                config.model_config.transducer = OfflineTransducerModelConfig {
                    encoder: Some(f.model.clone()),
                    decoder: Some(f.decoder.clone()),
                    joiner: Some(f.joiner.clone()),
                };
                config.model_config.model_type = Some("nemo_transducer".into());
            }
            Engine::Whisper => {
                config.model_config.whisper = OfflineWhisperModelConfig {
                    encoder: Some(f.model.clone()),
                    decoder: Some(f.decoder.clone()),
                    language: if f.language.is_empty() {
                        None
                    } else {
                        Some(f.language.clone())
                    },
                    ..Default::default()
                };
            }
        }
        config.model_config.tokens = Some(f.tokens.clone());
        config.model_config.num_threads = num_threads;

        let rec = OfflineRecognizer::create(&config)
            .ok_or("failed to create the recognizer (check model files)")?;
        Ok(Self {
            rec,
            workers: workers.max(1),
        })
    }

    /// Сплошной текст. Внутри — тот же проход со словами, что и `transcribe_words`.
    pub fn transcribe(
        &self,
        samples: &[f32],
        vad_model: Option<&str>,
        cancel: &AtomicBool,
        on_chunk: impl FnMut(usize, usize, &str),
    ) -> String {
        let words = self.transcribe_words(samples, vad_model, cancel, on_chunk);
        join_text(&words)
    }

    /// Файл целиком → слова с таймкодами. VAD-нарезка (если есть модель) или слепые окна.
    /// `on_chunk(done, total, partial)` — прогресс в семплах (сколько аудио уже распознано
    /// по порядку) + накопленный текст. Вызывается только на потоке вызывающего.
    pub fn transcribe_words(
        &self,
        samples: &[f32],
        vad_model: Option<&str>,
        cancel: &AtomicBool,
        mut on_chunk: impl FnMut(usize, usize, &str),
    ) -> Vec<Word> {
        let total = samples.len();
        let mut words: Vec<Word> = Vec::new();
        let mut partial = String::new();

        // Небольшая очередь: нарезчик не убегает далеко вперёд декодеров.
        let (seg_tx, seg_rx) = mpsc::sync_channel::<Seg>(self.workers * 2);
        let seg_rx = Mutex::new(seg_rx);
        let (res_tx, res_rx) = mpsc::channel::<(usize, usize, Vec<Word>)>();

        std::thread::scope(|s| {
            s.spawn(move || produce_segments(samples, vad_model, cancel, seg_tx));

            for _ in 0..self.workers {
                let res_tx = res_tx.clone();
                let seg_rx = &seg_rx;
                s.spawn(move || loop {
                    // лок держим только на время recv — следующий воркер ждёт свой сегмент
                    let next = seg_rx.lock().map(|rx| rx.recv());
                    let Ok(Ok(seg)) = next else { break };
                    let mut out = Vec::new();
                    if !cancel.load(Ordering::Relaxed) {
                        let off = seg.start as f32 / SR as f32;
                        self.decode_segment(
                            &samples[seg.start..seg.start + seg.len],
                            off,
                            &mut out,
                        );
                    }
                    if res_tx.send((seg.idx, seg.start + seg.len, out)).is_err() {
                        break;
                    }
                });
            }
            drop(res_tx);

            // Сборщик: результаты приходят вразнобой — выдаём строго по порядку сегментов.
            let mut pending: BTreeMap<usize, (usize, Vec<Word>)> = BTreeMap::new();
            let mut next = 0usize;
            for (idx, end, ws) in res_rx {
                pending.insert(idx, (end, ws));
                while let Some((end, ws)) = pending.remove(&next) {
                    next += 1;
                    for w in &ws {
                        if !partial.is_empty() {
                            partial.push(' ');
                        }
                        partial.push_str(&w.text);
                    }
                    words.extend(ws);
                    if !cancel.load(Ordering::Relaxed) {
                        on_chunk(end, total, &partial);
                    }
                }
            }
        });

        finalize_word_ends(&mut words);
        if !cancel.load(Ordering::Relaxed) {
            on_chunk(total, total, &partial);
        }
        words
    }

    /// Декод одного речевого куска (уже вырезанного) + добавление слов со сдвигом `offset`.
    fn decode_segment(&self, seg: &[f32], offset: f32, out: &mut Vec<Word>) {
        if seg.len() < (SR as usize) / 5 {
            return; // < 0.2с — пропускаем
        }
        if let Some(res) = self.decode(seg) {
            match res.timestamps {
                // Пословные таймкоды (GigaAM/Parakeet CTC/transducer) → точная привязка к спикерам.
                Some(times) if !times.is_empty() => push_words(&res.tokens, &times, offset, out),
                // Whisper даёт текст, но timestamps=Some(пусто) → берём текст сегмента целиком
                // (диаризация огрубляется до VAD-сегмента — у whisper пословных нет). Конец —
                // по концу сегмента: спикер выбирается по перекрытию со всем куском, а не по
                // первым 50 мс.
                _ => {
                    let t = res.text.trim();
                    if !t.is_empty() {
                        out.push(Word {
                            text: t.to_string(),
                            start: offset,
                            end: offset + seg.len() as f32 / SR as f32,
                        });
                    }
                }
            }
        }
    }

    fn decode(&self, seg: &[f32]) -> Option<sherpa_onnx::OfflineRecognizerResult> {
        let stream = self.rec.create_stream();
        stream.accept_waveform(SR, seg);
        self.rec.decode(&stream);
        stream.get_result()
    }
}

/// Нарезчик (свой поток): VAD-сегменты по мере появления или слепые окна по 20с.
/// Закрытие `tx` (выход из функции) = сигнал декодерам «сегментов больше не будет».
fn produce_segments(
    samples: &[f32],
    vad_model: Option<&str>,
    cancel: &AtomicBool,
    tx: mpsc::SyncSender<Seg>,
) {
    let total = samples.len();
    let mut idx = 0usize;

    if let Some(vad) = vad_model.and_then(make_vad) {
        // VAD отдаёт кусок как (start, n) в координатах всего потока → режем исходный буфер.
        let drain = |vad: &VoiceActivityDetector, idx: &mut usize| -> bool {
            while let Some(seg) = vad.front() {
                let start = (seg.start().max(0) as usize).min(total);
                let len = (seg.n().max(0) as usize).min(total - start);
                vad.pop();
                if tx
                    .send(Seg {
                        idx: *idx,
                        start,
                        len,
                    })
                    .is_err()
                {
                    return false;
                }
                *idx += 1;
            }
            true
        };
        let mut i = 0usize;
        while i < total {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let end = (i + VAD_WINDOW).min(total);
            vad.accept_waveform(&samples[i..end]);
            i = end;
            if !drain(&vad, &mut idx) {
                return;
            }
        }
        vad.flush();
        drain(&vad, &mut idx);
        return;
    }

    // фолбэк: слепые окна по 20с
    let win = CHUNK_SEC * SR as usize;
    let mut i = 0usize;
    while i < total {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let end = (i + win).min(total);
        if tx
            .send(Seg {
                idx,
                start: i,
                len: end - i,
            })
            .is_err()
        {
            return;
        }
        idx += 1;
        i = end;
    }
}

fn make_vad(model: &str) -> Option<VoiceActivityDetector> {
    let cfg = VadModelConfig {
        silero_vad: SileroVadModelConfig {
            model: Some(model.to_string()),
            threshold: 0.5,
            min_silence_duration: 0.5,
            min_speech_duration: 0.25,
            window_size: VAD_WINDOW as i32,
            max_speech_duration: VAD_MAX_SPEECH_SEC,
        },
        sample_rate: SR,
        // Silero на окне 512 семплов не параллелится (замер: 1/2/4 потока — одинаково),
        // а лишние потоки ORT только отъедают ядра у декодеров.
        num_threads: 1,
        provider: Some("cpu".to_string()),
        debug: false,
        ..Default::default()
    };
    VoiceActivityDetector::create(&cfg, 30.0)
}

fn join_text(words: &[Word]) -> String {
    words
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Гарантируем ненулевую ширину слова (для расчёта перекрытия при диаризации).
fn finalize_word_ends(words: &mut [Word]) {
    for w in words.iter_mut() {
        if w.end < w.start + 0.05 {
            w.end = w.start + 0.05;
        }
    }
}

/// Сборка слов из токенов. Граница слова — ведущий пробел токена: sherpa отдаёт
/// BPE-маркер «▁» уже как пробел (« ко», « Д»), у char-моделей пробел — отдельный токен.
/// Сырой «▁» тоже понимаем (на случай других моделей/версий).
/// `end` слова ≈ время последнего его токена (от `offset`).
fn push_words(tokens: &[String], times: &[f32], offset: f32, out: &mut Vec<Word>) {
    let mut cur = String::new();
    let mut start = offset;
    let mut cur_end = offset;
    for (tok, &t) in tokens.iter().zip(times.iter()) {
        let tt = offset + t;
        let body = tok.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{2581}');
        if body.len() != tok.len() && !cur.is_empty() {
            out.push(Word {
                text: std::mem::take(&mut cur),
                start,
                end: cur_end.max(start),
            });
        }
        if body.is_empty() {
            continue; // чистый разделитель слов
        }
        if cur.is_empty() {
            start = tt;
        }
        cur_end = tt;
        cur.push_str(body);
    }
    if !cur.is_empty() {
        out.push(Word {
            text: cur,
            start,
            end: cur_end.max(start),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn texts(out: &[Word]) -> Vec<&str> {
        out.iter().map(|w| w.text.as_str()).collect()
    }

    #[test]
    fn splits_on_leading_space_tokens() {
        // так sherpa отдаёт BPE GigaAM v3 / Parakeet: «▁» уже превращён в пробел
        let t = toks(&[
            " Д", "о", "б", "ро", "е", " у", "т", "ро", ",", " ко", "л", "ле", "ги", ".",
        ]);
        let times: Vec<f32> = (0..t.len()).map(|i| i as f32 * 0.1).collect();
        let mut out = Vec::new();
        push_words(&t, &times, 10.0, &mut out);
        assert_eq!(texts(&out), ["Доброе", "утро,", "коллеги."]);
        assert!((out[1].start - 10.5).abs() < 1e-4);
        assert!((out[1].end - 10.8).abs() < 1e-4);
    }

    #[test]
    fn splits_on_standalone_space_and_raw_bpe_marker() {
        let t = toks(&["▁на", "ш", "у", " ", "е", "же", "\u{2581}", "в", "с"]);
        let times: Vec<f32> = (0..t.len()).map(|i| i as f32).collect();
        let mut out = Vec::new();
        push_words(&t, &times, 0.0, &mut out);
        assert_eq!(texts(&out), ["нашу", "еже", "вс"]);
        assert_eq!(out[1].start, 4.0);
        assert_eq!(out[2].start, 7.0);
    }
}
