//! Быстрый тест движка (тот же код, что в приложении), без GUI.
//!   cargo run --example try_transcribe -- "C:\path\audio.mp4" [max_secs] [diarize]

use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use speakagent_lib::engine::{
    asr::{Asr, AsrFiles, Engine},
    decode, diarize, models, punct,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Режим загрузки модели: try_transcribe download <id>
    if args.get(1).map(|s| s == "download").unwrap_or(false) {
        use speakagent_lib::engine::models;
        let id = args.get(2).expect("download <id>");
        println!("скачиваю {id}…");
        models::download(id, |done, total| {
            if total > 0 {
                print!("\r  {:.0}%   ", done as f64 / total as f64 * 100.0);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
        })
        .expect("download");
        println!("\n✓ установлено. Каталог:");
        for m in models::list() {
            println!("  {:<14} {}", m.id, if m.installed { "✓" } else { "—" });
        }
        return;
    }

    // Режим теста PDF: try_transcribe pdf <out.pdf>
    if args.get(1).map(|s| s == "pdf").unwrap_or(false) {
        use speakagent_lib::engine::pdf::{save_pdf, PdfBlock};
        let out = args.get(2).map(|s| s.as_str()).unwrap_or("test.pdf");
        let blocks = vec![
            PdfBlock {
                heading: Some("Спикер 1".into()),
                time: Some("0:00:05".into()),
                body: "Привет! Это тестовая реплика на русском для проверки кириллицы в PDF.".into(),
            },
            PdfBlock {
                heading: Some("Спикер 2".into()),
                time: Some("0:00:12".into()),
                body: "Второй говорящий отвечает — проверяем перенос длинных строк и абзацы.".into(),
            },
        ];
        save_pdf("Тестовая расшифровка", &blocks, out).expect("pdf");
        println!("PDF сохранён: {out}");
        return;
    }

    let path =
        args.get(1).expect("usage: try_transcribe <audio> [max_secs] [diarize] [num_speakers] [threshold]");
    let max_secs: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(120.0);
    let do_diar = args.get(3).map(|s| s == "diarize").unwrap_or(false);
    let num_speakers: i32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
    let threshold: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.8);

    // dev-фолбэк: модели спайка в spike/models рядом с src-tauri (или задать SA_MODEL_DIR).
    // Прямые слэши — кроссплатформенно (Windows их тоже принимает).
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../spike/models");
    let model = format!("{base}/sherpa-onnx-nemo-ctc-giga-am-v2-russian-2025-04-19/model.int8.onnx");
    let tokens = format!("{base}/sherpa-onnx-nemo-ctc-giga-am-v2-russian-2025-04-19/tokens.txt");
    // Диаризация и VAD — через резолверы моделей (каталог данных → dev-фолбэк spike/models).
    let (seg, emb) = models::diarization().unwrap_or_else(|| {
        (
            format!("{base}/sherpa-onnx-pyannote-segmentation-3-0/model.onnx"),
            format!("{base}/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx"),
        )
    });
    let vad = models::vad();
    let vad_ref = vad.as_deref();

    println!(
        "VAD: {} · эмбеддер: {}",
        if vad_ref.is_some() { "вкл" } else { "нет (слепые окна)" },
        emb.rsplit(['\\', '/']).next().unwrap_or(&emb)
    );
    println!("декод {path} (первые {max_secs}с)…");
    let t = Instant::now();
    let samples = decode::decode_to_16k_mono_max(path, Some(max_secs)).expect("decode");
    let audio_sec = samples.len() as f32 / 16000.0;
    println!("  {audio_sec:.1}с аудио, декод за {:.1}с", t.elapsed().as_secs_f32());

    // SA_MODEL_DIR — тест произвольной transducer-модели (напр. GigaAM v3 punct):
    //   encoder.int8.onnx / decoder.onnx / joiner.onnx / tokens.txt в этой папке.
    let asr_files = if let Ok(dir) = std::env::var("SA_MODEL_DIR") {
        if std::path::Path::new(&format!("{dir}/model.int8.onnx")).exists() {
            println!("ASR (CTC) из SA_MODEL_DIR: {dir}");
            AsrFiles {
                engine: Engine::NemoCtc,
                model: format!("{dir}/model.int8.onnx"),
                decoder: String::new(),
                joiner: String::new(),
                tokens: format!("{dir}/tokens.txt"),
                language: String::new(),
            }
        } else {
            println!("ASR (transducer) из SA_MODEL_DIR: {dir}");
            AsrFiles {
                engine: Engine::Transducer,
                model: format!("{dir}/encoder.int8.onnx"),
                decoder: format!("{dir}/decoder.onnx"),
                joiner: format!("{dir}/joiner.onnx"),
                tokens: format!("{dir}/tokens.txt"),
                language: String::new(),
            }
        }
    } else if let Some(f) = models::active_asr_files() {
        println!("ASR (активная модель): {}", models::active_id());
        f
    } else {
        AsrFiles {
            engine: Engine::NemoCtc,
            model,
            decoder: String::new(),
            joiner: String::new(),
            tokens,
            language: String::new(),
        }
    };
    let asr = Asr::load(&asr_files, 16).expect("asr init");

    if do_diar {
        println!("диаризация (порог {threshold}, спикеров {}) + ASR + привязка…",
            if num_speakers > 0 { num_speakers.to_string() } else { "авто".into() });
        let t = Instant::now();
        let segs = diarize::diarize(&seg, &emb, &samples, 16, threshold, num_speakers).expect("diarize");
        let nspk_raw = segs.iter().map(|s| s.speaker).collect::<HashSet<_>>().len();
        let mut words = asr.transcribe_words(&samples, vad_ref, &AtomicBool::new(false), |_, _, _| {});
        // RUPunct только для русских моделей БЕЗ своей пунктуации (как в lib.rs).
        if models::needs_ru_punct(&models::active_id()) {
            if let Some(p) = models::punct().and_then(|(m, t)| punct::Punctuator::load(&m, &t)) {
                let texts: Vec<String> = words.iter().map(|w| w.text.clone()).collect();
                for (w, r) in words.iter_mut().zip(p.restore(&texts)) {
                    w.text = r;
                }
                println!("пунктуация RUPunct: применена");
            }
        }
        let out = diarize::words_to_replicas(&words, &segs);
        let dur = t.elapsed().as_secs_f32();
        println!(
            "  спикеров (сырых) {nspk_raw} · слов {} · за {dur:.1}с · x{:.1} realtime",
            words.len(),
            audio_sec / dur
        );
        let preview: String = out.chars().take(1400).collect();
        println!("\n--- РЕПЛИКИ ---\n{preview}");
    } else {
        println!("транскрипция…");
        let t = Instant::now();
        let text = asr.transcribe(&samples, vad_ref, &AtomicBool::new(false), |_, _, _| {});
        let asr_sec = t.elapsed().as_secs_f32();
        println!(
            "  за {asr_sec:.1}с · RTF {:.3} · x{:.1} realtime",
            asr_sec / audio_sec,
            audio_sec / asr_sec
        );
        let preview: String = text.chars().take(600).collect();
        println!("\n--- ТЕКСТ ---\n{preview}");
    }
}
