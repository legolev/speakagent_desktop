//! SpeakAgent Desktop — Tauri entry point.
//! Тонкая обёртка: окно + IPC + vibrancy. Вся логика — в модуле `engine` (ядро).

pub mod engine;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;

/// Флаги отмены активных расшифровок (по id задачи).
fn cancels() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static C: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Serialize)]
pub struct AppInfo {
    name: String,
    version: String,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "SpeakAgent".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

#[tauri::command]
fn file_info(path: String) -> Result<engine::audio::FileInfo, String> {
    engine::audio::file_info(&path)
}

/// Сохранить текстовый результат (TXT/MD/SRT) в выбранный файл.
#[tauri::command]
fn save_text(content: String, path: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Сгенерировать PDF (со встроенным шрифтом — кириллица) и сохранить.
#[tauri::command]
fn save_pdf(
    title: String,
    blocks: Vec<engine::pdf::PdfBlock>,
    path: String,
) -> Result<(), String> {
    engine::pdf::save_pdf(&title, &blocks, &path)
}

/// Открыть папку с данными приложения в системном проводнике.
#[tauri::command]
fn open_data_dir() -> Result<(), String> {
    let dir = engine::store::data_dir();
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    std::process::Command::new(program)
        .arg(&dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ─────────────── Железо / производительность ───────────────
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemInfo {
    physical_cores: usize,
    logical_cores: usize,
    ram_total_gb: f64,
    ram_available_gb: f64,
    /// RTF расшифровки БЕЗ диаризации (замер или эвристика).
    rtf_plain: f64,
    /// RTF расшифровки С диаризацией — дороже, калибруется отдельно.
    rtf_diar: f64,
    /// Фиксированный оверхед, сек (декод + загрузка моделей) — прибавлять к duration×rtf.
    overhead_sec: f64,
    measured: bool, // true — есть хотя бы один замер на этой машине
    speed: String,  // "fast" | "medium" | "slow"
    /// Видеокарта (лучшая из найденных), если есть.
    gpu_name: Option<String>,
    gpu_vram_gb: f64,
    /// Чем ускоряются «Итоги»: "gpu" | "cpu".
    llm_accel: String,
}

fn physical_cores() -> usize {
    sysinfo::System::new()
        .physical_core_count()
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        })
}

/// Эвристика RTF по ядрам (без диаризации) — до первого реального замера.
fn heuristic_rtf() -> f64 {
    match physical_cores() {
        0..=1 => 0.30,
        2 => 0.22,
        3..=4 => 0.16,
        5..=8 => 0.12,
        _ => 0.10,
    }
}

fn setting_f64(key: &str) -> Option<f64> {
    engine::store::get_setting(key)
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| *v > 0.0)
}

/// RTF по режимам: замеренные на этой машине (раздельно: диаризация заметно дороже)
/// или эвристика. Возвращает (rtf_plain, rtf_diar, measured).
fn estimate_rtf2() -> (f64, f64, bool) {
    let plain = setting_f64("rtf_plain");
    let diar = setting_f64("rtf_diar");
    // старый общий ключ (установки до раздельной калибровки) — как посев для обоих
    let legacy = setting_f64("measured_rtf");
    let h = heuristic_rtf();
    let p = plain.or(legacy).unwrap_or(h);
    // диаризация ≈ ×1.8 к обычной, пока не замерена отдельно
    let d = diar.or_else(|| plain.or(legacy).map(|v| v * 1.8)).unwrap_or(h * 1.8);
    (p, d, plain.is_some() || diar.is_some() || legacy.is_some())
}

/// Оверхед, не зависящий от длины записи: декод + загрузка моделей (замер или дефолт).
fn estimate_overhead() -> f64 {
    setting_f64("measured_overhead").unwrap_or(25.0)
}

#[tauri::command]
fn system_info() -> SystemInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let physical = physical_cores();
    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let (rtf_plain, rtf_diar, measured) = estimate_rtf2();
    let speed = if rtf_plain < 0.11 {
        "fast"
    } else if rtf_plain < 0.22 {
        "medium"
    } else {
        "slow"
    };
    let gpu = engine::gpu::best_gpu();
    let llm_accel = if engine::gpu::should_offload().is_some() {
        "gpu"
    } else {
        "cpu"
    };
    SystemInfo {
        physical_cores: physical,
        logical_cores: logical,
        ram_total_gb: sys.total_memory() as f64 / 1e9,
        ram_available_gb: sys.available_memory() as f64 / 1e9,
        rtf_plain,
        rtf_diar,
        overhead_sec: estimate_overhead(),
        measured,
        speed: speed.into(),
        gpu_vram_gb: gpu.as_ref().map(|g| g.vram_gb()).unwrap_or(0.0),
        gpu_name: gpu.map(|g| g.name),
        llm_accel: llm_accel.into(),
    }
}

#[tauri::command]
fn probe_duration(path: String) -> Option<f32> {
    engine::decode::probe_duration(&path)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Usage {
    cpu_pct: f32,
    ram_used_pct: f32,
}

#[tauri::command]
fn resource_usage() -> Usage {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_usage();
    std::thread::sleep(std::time::Duration::from_millis(120));
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let total = sys.total_memory() as f32;
    Usage {
        cpu_pct: sys.global_cpu_usage(),
        ram_used_pct: if total > 0.0 {
            (1.0 - sys.available_memory() as f32 / total) * 100.0
        } else {
            0.0
        },
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    stage: String, // "decoding" | "diarizing" | "transcribing" | "done"
    done: usize,
    total: usize,
    partial: String,
    /// Длительность записи, сек (известна после декода; 0 до того) — для истории.
    audio_sec: f32,
}

/// Порог кластеризации диаризации (используется, когда число спикеров неизвестно).
/// Выше → меньше спикеров. Подбирается на реальных файлах; при известном числе
/// спикеров (num_speakers) не используется вовсе.
const CLUSTER_THRESHOLD: f32 = 0.8;

/// Расшифровка файла: декод → (опц. диаризация) → GigaAM. Прогресс через Channel.
/// `num_speakers`: Some(n>0) — точное число говорящих (надёжнее авто); None/0 — авто.
/// Тяжёлая CPU-работа уходит в blocking-поток, чтобы не морозить UI.
#[tauri::command]
async fn transcribe(
    path: String,
    diarize: bool,
    num_speakers: Option<i32>,
    job_id: String,
    on_progress: tauri::ipc::Channel<Progress>,
) -> Result<String, String> {
    let cancel = Arc::new(AtomicBool::new(false));
    cancels().lock().unwrap().insert(job_id.clone(), cancel.clone());

    let out = tauri::async_runtime::spawn_blocking(move || {
        let send = |stage: &str, done: usize, total: usize, partial: &str| {
            let _ = on_progress.send(Progress {
                stage: stage.into(),
                done,
                total,
                partial: partial.to_string(),
                audio_sec: 0.0,
            });
        };

        let started = std::time::Instant::now();
        send("decoding", 0, 0, "");
        let samples = engine::decode::decode_to_16k_mono(&path)?;
        let audio_sec = samples.len() as f64 / 16000.0;

        let files = engine::models::active_asr_files().ok_or(
            "Модель распознавания не установлена. Откройте «Настройки» и выберите модель.",
        )?;
        let asr = engine::asr::Asr::load(&files, num_threads())?;
        // фиксированная часть (декод + загрузка модели) — калибруем отдельно от RTF
        let overhead_sec = started.elapsed().as_secs_f64();
        let processing_started = std::time::Instant::now();
        // VAD для нарезки по речи (границы в тишине). Нет модели → слепые окна.
        let vad = engine::models::vad();
        let vad_ref = vad.as_deref();

        // Диаризация (если нужна) — по всему файлу, до ASR.
        let segs = if diarize {
            send("diarizing", 0, 0, "");
            let (seg, emb) = engine::models::diarization().ok_or(
                "Модели для распознавания говорящих не установлены. Скачайте их в «Настройках».",
            )?;
            let n_spk = num_speakers.unwrap_or(0).max(0);
            Some(engine::diarize::diarize(
                &seg,
                &emb,
                &samples,
                num_threads(),
                CLUSTER_THRESHOLD,
                n_spk,
            )?)
        } else {
            None
        };

        // Один проход ASR по всему файлу со словами+таймкодами (полный контекст).
        let mut words = asr.transcribe_words(&samples, vad_ref, &cancel, |done, total, partial| {
            send("transcribing", done, total, partial);
        });

        // Внешняя пунктуация (RUPunct) — fallback только для русских моделей БЕЗ своей
        // пунктуации. GigaAM v3 (дефолт) пунктуирует сам, Whisper тоже — тогда шаг пропускается.
        if !cancel.load(Ordering::Relaxed)
            && engine::models::needs_ru_punct(&engine::models::active_id())
        {
            if let Some(p) = engine::models::punct()
                .and_then(|(m, t)| engine::punct::Punctuator::load(&m, &t))
            {
                send("punctuating", 0, 0, "");
                let texts: Vec<String> = words.iter().map(|w| w.text.clone()).collect();
                for (w, r) in words.iter_mut().zip(p.restore(&texts)) {
                    w.text = r;
                }
            }
        }

        // Привязка слов к спикерам (диаризация) либо плоский текст.
        let out = match &segs {
            Some(segs) => engine::diarize::words_to_replicas(&words, segs),
            None => words
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        };

        // Калибровка ETA (если не отменено): RTF — раздельно по режимам (диаризация
        // заметно дороже), оверхед (декод+загрузка моделей) — своим ключом.
        if !cancel.load(Ordering::Relaxed) && audio_sec > 30.0 {
            let rtf = processing_started.elapsed().as_secs_f64() / audio_sec;
            let key = if diarize { "rtf_diar" } else { "rtf_plain" };
            let _ = engine::store::set_setting(key, &format!("{rtf:.4}"));
            let _ = engine::store::set_setting("measured_overhead", &format!("{overhead_sec:.1}"));
        }

        // финальное сообщение несёт реальную длительность записи (для истории)
        let _ = on_progress.send(Progress {
            stage: "done".into(),
            done: 1,
            total: 1,
            partial: out.clone(),
            audio_sec: audio_sec as f32,
        });
        Ok::<String, String>(out)
    })
    .await
    .map_err(|e| e.to_string())?;

    cancels().lock().unwrap().remove(&job_id);
    out
}

/// Отменить активную расшифровку (флаг проверяется в цикле ASR).
#[tauri::command]
fn cancel_transcribe(job_id: String) {
    if let Some(f) = cancels().lock().unwrap().get(&job_id) {
        f.store(true, Ordering::Relaxed);
    }
}

fn num_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| (n.get() as i32).clamp(1, 16))
        .unwrap_or(4)
}

// ─────────────── История (SQLite) ───────────────
#[tauri::command]
fn list_jobs() -> Result<Vec<engine::store::StoredJob>, String> {
    engine::store::list()
}

#[tauri::command]
fn save_job(job: engine::store::StoredJob) -> Result<(), String> {
    engine::store::save(&job)
}

#[tauri::command]
fn delete_job(id: String) -> Result<(), String> {
    engine::store::delete(&id)
}

#[tauri::command]
fn clear_jobs() -> Result<(), String> {
    engine::store::clear()
}

// ─────────────── Модели ───────────────
#[tauri::command]
fn list_models() -> Vec<engine::models::ModelInfo> {
    engine::models::list()
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DlProgress {
    done: u64,
    total: u64,
}

#[tauri::command]
async fn download_model(
    id: String,
    on_progress: tauri::ipc::Channel<DlProgress>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        engine::models::download(&id, |done, total| {
            let _ = on_progress.send(DlProgress { done, total });
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Готова ли активная модель к расшифровке (для онбординга при первом запуске).
#[tauri::command]
fn is_ready() -> bool {
    engine::models::active_asr_files().is_some()
}

#[tauri::command]
fn active_model() -> String {
    engine::models::active_id()
}

#[tauri::command]
fn set_active_model(id: String) -> Result<(), String> {
    engine::models::set_active(&id)
}

/// Фоновая докачка инфраструктурных моделей (диаризация + ffmpeg) — молча.
/// Сюда же — тихий GPU-апгрейд движка итогов (CPU-сборка → Vulkan, ~32 МБ):
/// у уже установленных llm_ready=true, и флоу «Скачать помощника» не запустится.
#[tauri::command]
async fn ensure_core() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        for id in engine::models::missing_core() {
            let _ = engine::models::download(&id, |_, _| {});
        }
        if engine::models::llama_needs_gpu_upgrade() {
            let _ = engine::models::download("llama", |_, _| {});
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ─────────────── «Итоги встречи» (локальный LLM) ───────────────

/// Готова ли фича (движок + активная модель установлены).
#[tauri::command]
fn llm_ready() -> bool {
    engine::llm::is_ready()
}

/// Докачать всё для «Итогов» (движок + активная LLM-модель) с прогрессом.
#[tauri::command]
async fn ensure_llm(on_progress: tauri::ipc::Channel<DlProgress>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        for id in engine::models::missing_llm() {
            engine::models::download(&id, |done, total| {
                let _ = on_progress.send(DlProgress { done, total });
            })?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LlmProgress {
    stage: String, // "starting" | "reading" | "writing"
    done: u32,
    total: u32,
    partial: String,
}

/// Составить артефакт («summary» | «business» | «interview» | «todo») по записи из истории.
/// Результат сохраняется в job_results; дайджест длинных записей кэшируется там же.
#[tauri::command]
async fn llm_generate(
    job_id: String,
    kind: String,
    on_progress: tauri::ipc::Channel<LlmProgress>,
) -> Result<String, String> {
    let rkind = engine::llm::ResultKind::parse(&kind).ok_or("неизвестный тип итога")?;
    let cancel = Arc::new(AtomicBool::new(false));
    let ckey = format!("llm:{job_id}");
    cancels().lock().unwrap().insert(ckey.clone(), cancel.clone());

    let out = tauri::async_runtime::spawn_blocking(move || {
        let job = engine::store::get(&job_id).ok_or("запись не найдена в истории")?;
        if job.text.trim().is_empty() {
            return Err("в записи нет текста".into());
        }
        let transcript = engine::llm::prepare_transcript(&job.text, &job.speakers);
        let digest = engine::store::digest_for(&job_id);

        let gen = engine::llm::generate(rkind, &transcript, digest.as_deref(), &cancel, |p| {
            let _ = on_progress.send(LlmProgress {
                stage: p.stage.into(),
                done: p.done,
                total: p.total,
                partial: p.partial,
            });
        })?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let model = engine::models::active_llm_id();
        if let Some(d) = &gen.digest {
            let _ = engine::store::save_result(&engine::store::JobResult {
                job_id: job_id.clone(),
                kind: "digest".into(),
                text: d.clone(),
                model: model.clone(),
                created_at: now,
            });
        }
        engine::store::save_result(&engine::store::JobResult {
            job_id,
            kind: rkind.as_str().into(),
            text: gen.text.clone(),
            model,
            created_at: now,
        })?;
        Ok::<String, String>(gen.text)
    })
    .await
    .map_err(|e| e.to_string())?;

    cancels().lock().unwrap().remove(&ckey);
    out
}

/// Отменить генерацию итога по записи.
#[tauri::command]
fn cancel_llm(job_id: String) {
    if let Some(f) = cancels().lock().unwrap().get(&format!("llm:{job_id}")) {
        f.store(true, Ordering::Relaxed);
    }
}

/// Сохранённые итоги записи (без служебного дайджеста).
#[tauri::command]
fn list_results(job_id: String) -> Result<Vec<engine::store::JobResult>, String> {
    engine::store::results_for(&job_id)
}

/// Пользовательская правка итога (переключение чекбоксов задач и т.п.).
#[tauri::command]
fn save_result_text(job_id: String, kind: String, text: String) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    engine::store::save_result(&engine::store::JobResult {
        job_id,
        kind,
        text,
        model: engine::models::active_llm_id(),
        created_at: now,
    })
}

/// Готовый запрос для внешнего ИИ: промпт + расшифровка (с именами спикеров).
/// Запасной путь для слабых машин — пользователь копирует и несёт в любой ИИ-чат.
#[tauri::command]
fn llm_export_prompt(job_id: String, kind: String) -> Result<String, String> {
    let rkind = engine::llm::ResultKind::parse(&kind).ok_or("неизвестный тип итога")?;
    let job = engine::store::get(&job_id).ok_or("запись не найдена")?;
    let transcript = engine::llm::prepare_transcript(&job.text, &job.speakers);
    Ok(format!(
        "{}\n\n---\n\nРасшифровка:\n\n{}",
        engine::llm::prompt_text(rkind),
        transcript
    ))
}

/// Придумать короткое название записи (только когда модель уже в памяти —
/// вызывается после первой генерации итога, стоит секунды).
#[tauri::command]
async fn llm_display_name(job_id: String) -> Result<String, String> {
    if !engine::llm::is_warm() {
        return Err("помощник не запущен".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let job = engine::store::get(&job_id).ok_or("запись не найдена")?;
        let transcript = engine::llm::prepare_transcript(&job.text, &job.speakers);
        let name = engine::llm::display_name(&transcript, &AtomicBool::new(false))?;
        if name.trim().is_empty() || name.chars().count() > 80 {
            return Err("название не получилось".into());
        }
        Ok(name)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn active_llm_model() -> String {
    engine::models::active_llm_id()
}

#[tauri::command]
fn set_active_llm_model(id: String) -> Result<(), String> {
    engine::llm::shutdown(); // сменилась модель — старый сервер больше не нужен
    engine::models::set_active_llm(&id)
}

// ─────────────── ИИ-провайдер: локальный движок ↔ облако ───────────────

/// Какой бэкенд «Итогов»: "local" (llama-server) или "cloud" (OpenAI-совместимый провайдер).
#[tauri::command]
fn llm_backend() -> String {
    engine::store::get_setting("llm_backend").unwrap_or_else(|| "local".into())
}

#[tauri::command]
fn set_llm_backend(mode: String) -> Result<(), String> {
    if mode != "local" && mode != "cloud" {
        return Err("неизвестный режим".into());
    }
    if mode == "cloud" {
        engine::llm::shutdown(); // локальный сервер больше не нужен
    }
    engine::store::set_setting("llm_backend", &mode)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CloudConfig {
    url: String,
    model: String,
    key: String,
}

/// Текущие настройки облачного провайдера (с дефолтами для полей формы).
#[tauri::command]
fn cloud_config() -> CloudConfig {
    CloudConfig {
        url: engine::store::get_setting("cloud_url")
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".into()),
        model: engine::store::get_setting("cloud_model")
            .unwrap_or_else(|| "openai/gpt-4o-mini".into()),
        key: engine::store::get_setting("cloud_key").unwrap_or_default(),
    }
}

#[tauri::command]
fn set_cloud_config(url: String, model: String, key: String) -> Result<(), String> {
    engine::store::set_setting("cloud_url", url.trim())?;
    engine::store::set_setting("cloud_model", model.trim())?;
    engine::store::set_setting("cloud_key", key.trim())
}

/// Проверить связь с облачным провайдером (мини-запрос). Возвращает ответ модели.
#[tauri::command]
async fn test_cloud(url: String, model: String, key: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || engine::llm::cloud_test(&url, &model, &key))
        .await
        .map_err(|e| e.to_string())?
}

// ─────────────── Диагностика / ID устройства ───────────────

/// Устойчивый хэш (FNV-1a 64) — стабилен между версиями Rust, в отличие от DefaultHasher.
fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Стабильный ID устройства: аппаратный machine-id ОС → FNV-1a → короткий неизменяемый
/// отпечаток `XXXX-XXXX-XXXX-XXXX`. Не меняется между запусками/переустановками; сырой
/// аппаратный UUID наружу не отдаём. Фолбэк — сгенерированный и сохранённый ID.
#[tauri::command]
fn device_id() -> String {
    let base = machine_uid::get()
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if let Some(v) = engine::store::get_setting("device_fallback_id") {
                return v;
            }
            let seed = format!("{:?}", std::time::SystemTime::now());
            let id = format!("{:016X}", fnv1a(&seed));
            let _ = engine::store::set_setting("device_fallback_id", &id);
            id
        });
    let hex = format!("{:016X}", fnv1a(&format!("speakagent:{}", base.trim())));
    format!("{}-{}-{}-{}", &hex[0..4], &hex[4..8], &hex[8..12], &hex[12..16])
}

/// Служебная информация одним текстом — чтобы пользователь мог скопировать/отправить в баг-репорт.
#[tauri::command]
fn diagnostics() -> String {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let ram = sys.total_memory() as f64 / 1e9;
    let gpu = engine::gpu::best_gpu();
    let accel = if engine::gpu::should_offload().is_some() { "GPU" } else { "CPU" };
    let backend = engine::store::get_setting("llm_backend").unwrap_or_else(|| "local".into());
    let installed: Vec<String> = engine::models::list()
        .into_iter()
        .filter(|m| m.installed)
        .map(|m| m.id)
        .collect();
    format!(
        "SpeakAgent Desktop — служебная информация\n\
         Версия: {ver}\n\
         ID устройства: {dev}\n\
         ОС: {os} {arch}\n\
         Железо: {cores} ядер, {ram:.1} ГБ ОЗУ\n\
         Видеокарта: {gpuname} · ускорение «Итогов»: {accel}\n\
         ASR: {asr} · готова: {ready}\n\
         ИИ-функции: {backend} · модель {llm}\n\
         ffmpeg: {ffmpeg}\n\
         Скачанные модели: {installed}",
        ver = env!("CARGO_PKG_VERSION"),
        dev = device_id(),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        cores = physical_cores(),
        gpuname = gpu.map(|g| g.name).unwrap_or_else(|| "нет".into()),
        asr = engine::models::active_id(),
        ready = engine::models::active_asr_files().is_some(),
        llm = engine::models::active_llm_id(),
        ffmpeg = engine::models::ffmpeg().is_some(),
        installed = installed.join(", "),
    )
}

/// Показать файл в системном менеджере (Finder/Explorer) с выделением.
#[tauri::command]
fn reveal_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg("-R").arg(&path);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("explorer");
        c.arg(format!("/select,{path}"));
        c
    };
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let dir = std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| ".".into());
        let mut c = std::process::Command::new("xdg-open");
        c.arg(dir);
        c
    };
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Открыть внешний http(s)-адрес в системном браузере (для ссылки на репозиторий и т.п.).
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("недопустимый адрес".into());
    }
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    std::process::Command::new(program)
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            apply_window_effects(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            file_info,
            save_text,
            save_pdf,
            open_data_dir,
            system_info,
            probe_duration,
            resource_usage,
            transcribe,
            cancel_transcribe,
            list_jobs,
            save_job,
            delete_job,
            clear_jobs,
            list_models,
            download_model,
            is_ready,
            active_model,
            set_active_model,
            ensure_core,
            llm_ready,
            ensure_llm,
            llm_generate,
            cancel_llm,
            list_results,
            save_result_text,
            llm_export_prompt,
            llm_display_name,
            active_llm_model,
            set_active_llm_model,
            llm_backend,
            set_llm_backend,
            cloud_config,
            set_cloud_config,
            test_cloud,
            open_url,
            reveal_file,
            device_id,
            diagnostics
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app, event| {
            // гасим sidecar llama-server вместе с приложением (иначе повиснет в фоне)
            if matches!(event, tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }) {
                engine::llm::shutdown();
            }
        });
}

/// Нативный «стеклянный» фон окна: Mica на Windows 11, vibrancy на macOS.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn apply_window_effects(app: &tauri::App) {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("main") {
        #[cfg(target_os = "windows")]
        let _ = window_vibrancy::apply_mica(&win, Some(true));
        #[cfg(target_os = "macos")]
        let _ = window_vibrancy::apply_vibrancy(
            &win,
            window_vibrancy::NSVisualEffectMaterial::HudWindow,
            None,
            None,
        );
    }
}
