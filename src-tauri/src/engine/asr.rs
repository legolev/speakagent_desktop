//! ASR через sherpa-onnx. Поддержка GigaAM (CTC), Parakeet (transducer), Whisper.
//! Нарезка длинного аудио — по VAD (Silero), чтобы границы падали в тишину, а не
//! посреди слова (иначе «мусор» на стыках). Если модель VAD недоступна — слепые окна.

use std::sync::atomic::{AtomicBool, Ordering};

use sherpa_onnx::{
    OfflineNemoEncDecCtcModelConfig, OfflineRecognizer, OfflineRecognizerConfig,
    OfflineTransducerModelConfig, OfflineWhisperModelConfig, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};

const SR: i32 = 16000;
const CHUNK_SEC: usize = 20; // слепой фолбэк, если нет VAD
const VAD_WINDOW: usize = 512; // окно Silero VAD (samples)
const VAD_MAX_SPEECH_SEC: f32 = 20.0; // не даём offline-CTC слишком длинный кусок

/// Тип движка модели.
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

/// Распознаватель с загруженной моделью. Создаётся один раз, переиспользуется.
pub struct Asr {
    rec: OfflineRecognizer,
    threads: i32,
}

impl Asr {
    pub fn load(f: &AsrFiles, num_threads: i32) -> Result<Self, String> {
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
            .ok_or("не удалось создать recognizer (проверь файлы модели)")?;
        Ok(Self {
            rec,
            threads: num_threads,
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
        words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Файл целиком → слова с таймкодами. VAD-нарезка (если есть модель) или слепые окна.
    /// `on_chunk(done, total, partial)` — прогресс (в семплах).
    pub fn transcribe_words(
        &self,
        samples: &[f32],
        vad_model: Option<&str>,
        cancel: &AtomicBool,
        mut on_chunk: impl FnMut(usize, usize, &str),
    ) -> Vec<Word> {
        let mut words: Vec<Word> = Vec::new();

        // ── путь 1: нарезка по VAD (границы в тишине) ──
        if let Some(vm) = vad_model {
            if let Some(vad) = self.make_vad(vm) {
                let total = samples.len();
                let mut i = 0usize;
                while i < total {
                    if cancel.load(Ordering::Relaxed) {
                        finalize_word_ends(&mut words);
                        return words;
                    }
                    let end = (i + VAD_WINDOW).min(total);
                    vad.accept_waveform(&samples[i..end]);
                    i = end;
                    while let Some(seg) = vad.front() {
                        let off = seg.start() as f32 / SR as f32;
                        self.decode_segment(seg.samples(), off, &mut words);
                        vad.pop();
                        on_chunk(i, total, &join_text(&words));
                    }
                }
                vad.flush();
                while let Some(seg) = vad.front() {
                    let off = seg.start() as f32 / SR as f32;
                    self.decode_segment(seg.samples(), off, &mut words);
                    vad.pop();
                }
                finalize_word_ends(&mut words);
                on_chunk(total, total, &join_text(&words));
                return words;
            }
        }

        // ── путь 2 (фолбэк): слепые окна по 20с ──
        let win = CHUNK_SEC * SR as usize;
        let total = samples.len().div_ceil(win).max(1);
        let mut done = 0usize;
        let mut i = 0usize;
        while i < samples.len() {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let end = (i + win).min(samples.len());
            let off = i as f32 / SR as f32;
            self.decode_segment(&samples[i..end], off, &mut words);
            i = end;
            done += 1;
            if done % 2 == 0 || done == total {
                on_chunk(done, total, &join_text(&words));
            }
        }
        finalize_word_ends(&mut words);
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
                // (диаризация огрубляется до границы VAD-сегмента — у whisper пословных нет).
                _ => {
                    let t = res.text.trim();
                    if !t.is_empty() {
                        out.push(Word {
                            text: t.to_string(),
                            start: offset,
                            end: offset,
                        });
                    }
                }
            }
        }
    }

    fn make_vad(&self, model: &str) -> Option<VoiceActivityDetector> {
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
            num_threads: self.threads,
            provider: Some("cpu".to_string()),
            debug: false,
            ..Default::default()
        };
        VoiceActivityDetector::create(&cfg, 30.0)
    }

    fn decode(&self, seg: &[f32]) -> Option<sherpa_onnx::OfflineRecognizerResult> {
        let stream = self.rec.create_stream();
        stream.accept_waveform(SR, seg);
        self.rec.decode(&stream);
        stream.get_result()
    }
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

/// Сборка слов из токенов. Поддержка char-моделей (пробел-токен) и BPE (префикс ▁).
/// `end` слова ≈ время последнего его токена (от `offset`).
fn push_words(tokens: &[String], times: &[f32], offset: f32, out: &mut Vec<Word>) {
    let mut cur = String::new();
    let mut start = offset;
    let mut cur_end = offset;
    for (tok, &t) in tokens.iter().zip(times.iter()) {
        let tt = offset + t;
        if tok == " " || tok.chars().all(char::is_whitespace) {
            // char-модель: пробел = граница слова
            if !cur.is_empty() {
                out.push(Word {
                    text: std::mem::take(&mut cur),
                    start,
                    end: cur_end.max(start),
                });
            }
        } else if let Some(rest) = tok.strip_prefix('\u{2581}') {
            // BPE: ▁ = начало нового слова
            if !cur.is_empty() {
                out.push(Word {
                    text: std::mem::take(&mut cur),
                    start,
                    end: cur_end.max(start),
                });
            }
            start = tt;
            cur_end = tt;
            cur.push_str(rest);
        } else {
            if cur.is_empty() {
                start = tt;
            }
            cur_end = tt;
            cur.push_str(tok);
        }
    }
    if !cur.is_empty() {
        out.push(Word {
            text: cur,
            start,
            end: cur_end.max(start),
        });
    }
}
