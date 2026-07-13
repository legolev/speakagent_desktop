//! SpeakAgent Desktop — Tauri entry point.
//! Тонкая обёртка: окно + IPC + vibrancy. Вся логика — в модуле `engine` (ядро).

pub mod engine;
pub mod keys;
pub mod mcp;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

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

// ═══════════════════════ Диктовка (push-to-talk) ═══════════════════════

/// По умолчанию — правый Shift (одна клавиша; сам по себе ничего не печатает → удобно
/// для push-to-talk). Формат — «+»-склейка имён клавиш rdev (напр. "ControlLeft+Space").
const DEFAULT_HOTKEY: &str = "ShiftRight";

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn get_bool(key: &str, default: bool) -> bool {
    engine::store::get_setting(key)
        .map(|v| v != "0")
        .unwrap_or(default)
}

/// Настоящий выход (через трей) vs. сворачивание при закрытии окна.
fn really_quit() -> &'static AtomicBool {
    static Q: OnceLock<AtomicBool> = OnceLock::new();
    Q.get_or_init(|| AtomicBool::new(false))
}

/// Набор клавиш-триггера диктовки (имена rdev). Пусто → хоткей выключен.
fn dict_trigger() -> &'static Mutex<Vec<String>> {
    static T: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(Vec::new()))
}

fn set_trigger(spec: &str) {
    let keys: Vec<String> = spec
        .split('+')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    *dict_trigger().lock().unwrap() = keys;
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DictState {
    recording: bool,
    processing: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DictationConfig {
    hotkey: String,
    mode: String, // "hold" | "toggle"
    autopaste: bool,
    sound: bool,
    input_device: String, // "" = устройство по умолчанию
    model: String,        // "" = как активная ASR
    lang: String,
}

#[tauri::command]
fn dictation_config() -> DictationConfig {
    DictationConfig {
        hotkey: engine::store::get_setting("dict_keys").unwrap_or_else(|| DEFAULT_HOTKEY.into()),
        mode: engine::store::get_setting("dict_mode").unwrap_or_else(|| "hold".into()),
        autopaste: get_bool("dict_autopaste", true),
        sound: get_bool("dict_sound", true),
        input_device: engine::store::get_setting("dict_input_device").unwrap_or_default(),
        model: engine::store::get_setting("dict_asr").unwrap_or_default(),
        lang: engine::store::get_setting("dict_lang").unwrap_or_default(),
    }
}

#[tauri::command]
fn set_dictation_config(config: DictationConfig) -> Result<(), String> {
    engine::store::set_setting("dict_keys", &config.hotkey)?;
    engine::store::set_setting("dict_mode", &config.mode)?;
    engine::store::set_setting("dict_autopaste", if config.autopaste { "1" } else { "0" })?;
    engine::store::set_setting("dict_sound", if config.sound { "1" } else { "0" })?;
    engine::store::set_setting("dict_input_device", &config.input_device)?;
    engine::models::set_dict_asr(&config.model)?;
    engine::store::set_setting("dict_lang", &config.lang)?;
    set_trigger(&config.hotkey); // применяется на лету — слушатель читает набор клавиш
    Ok(())
}

#[tauri::command]
fn list_input_devices() -> Vec<String> {
    engine::dictation::list_input_devices()
}

#[tauri::command]
fn dictation_recording() -> bool {
    engine::dictation::is_recording()
}

#[tauri::command]
fn dictation_start(app: AppHandle) {
    begin_dictation(&app);
}

#[tauri::command]
fn dictation_stop(app: AppHandle) {
    finish_dictation(&app);
}

#[tauri::command]
fn list_dictations() -> Result<Vec<engine::store::StoredDictation>, String> {
    engine::store::list_dictations()
}

#[tauri::command]
fn delete_dictation(id: String) -> Result<(), String> {
    engine::store::delete_dictation(&id)
}

#[tauri::command]
fn clear_dictations() -> Result<(), String> {
    engine::store::clear_dictations()
}

/// Глобальный триггер диктовки — собственный слушатель клавиш (`keys::listen`).
/// Push-to-talk по одиночной клавише/модификатору (напр. правый Shift) или комбинации,
/// работает ГЛОБАЛЬНО (когда активно другое окно). Набор клавиш читается из `dict_trigger()`
/// на каждом событии → смена хоткея применяется на лету. macOS: CGEventTap (нужен Input
/// Monitoring); Windows: WH_KEYBOARD_LL.
fn start_key_listener(app: &AppHandle) {
    static STARTED: OnceLock<AtomicBool> = OnceLock::new();
    if STARTED
        .get_or_init(|| AtomicBool::new(false))
        .swap(true, Ordering::SeqCst)
    {
        return;
    }
    let app = app.clone();
    let pressed = Arc::new(Mutex::new(std::collections::HashSet::<String>::new()));
    let armed = Arc::new(AtomicBool::new(false));
    keys::listen(move |name, down| {
        {
            let mut set = pressed.lock().unwrap();
            if down {
                set.insert(name.to_string());
            } else {
                set.remove(name);
            }
        }
        let all_down = {
            let set = pressed.lock().unwrap();
            let trig = dict_trigger().lock().unwrap();
            !trig.is_empty() && trig.iter().all(|t| set.contains(t))
        };
        if all_down {
            if !armed.swap(true, Ordering::SeqCst) {
                on_trigger_down(&app);
            }
        } else if armed.swap(false, Ordering::SeqCst) {
            on_trigger_up(&app);
        }
    });
}

/// Триггер зажат — старт (или toggle-переключение). Тяжёлую работу уводим с потока
/// event-tap, чтобы не тормозить обработку системных событий.
fn on_trigger_down(app: &AppHandle) {
    let toggle = engine::store::get_setting("dict_mode").as_deref() == Some("toggle");
    let app = app.clone();
    std::thread::spawn(move || {
        if toggle {
            if engine::dictation::is_recording() {
                finish_dictation(&app);
            } else {
                begin_dictation(&app);
            }
        } else {
            begin_dictation(&app);
        }
    });
}

/// Триггер отпущен — стоп (только в режиме hold).
fn on_trigger_up(app: &AppHandle) {
    if engine::store::get_setting("dict_mode").as_deref() == Some("toggle") {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || finish_dictation(&app));
}

/// Начать запись: микрофон + звук + оверлей.
fn begin_dictation(app: &AppHandle) {
    if engine::dictation::is_recording() {
        return;
    }
    let device = engine::store::get_setting("dict_input_device").unwrap_or_default();
    match engine::dictation::start(&device) {
        Ok(()) => {
            if get_bool("dict_sound", true) {
                engine::sound::play_start_cue();
            }
            show_overlay(app);
            let _ = app.emit(
                "dictation:state",
                DictState {
                    recording: true,
                    processing: false,
                },
            );
            refresh_tray(app);
        }
        Err(e) => {
            let _ = app.emit("dictation:error", e);
        }
    }
}

/// Остановить запись: распознать (в фоне) → буфер обмена → авто-вставка → история.
fn finish_dictation(app: &AppHandle) {
    if !engine::dictation::is_recording() {
        return;
    }
    let pcm = match engine::dictation::stop() {
        Ok(p) => p,
        Err(_) => {
            hide_overlay(app);
            return;
        }
    };
    if get_bool("dict_sound", true) {
        engine::sound::play_stop_cue();
    }
    let _ = app.emit(
        "dictation:state",
        DictState {
            recording: false,
            processing: true,
        },
    );
    refresh_tray(app);

    let app = app.clone();
    std::thread::spawn(move || {
        let dur = pcm.len() as f64 / 16000.0;
        match engine::dictation::transcribe(pcm) {
            Ok(text) => {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    use tauri_plugin_clipboard_manager::ClipboardExt;
                    let _ = app.clipboard().write_text(text.clone());
                    if get_bool("dict_autopaste", true) {
                        // Пауза (на воркере): буфер должен записаться, а целевое окно —
                        // вернуть фокус. Саму эмуляцию Cmd/Ctrl+V гоним ТОЛЬКО с главного
                        // потока — enigo дёргает TSM/HIToolbox, который ассертит main-thread
                        // на macOS (иначе SIGTRAP).
                        std::thread::sleep(std::time::Duration::from_millis(120));
                        let _ = app.run_on_main_thread(paste_to_cursor);
                    }
                    let entry = engine::store::StoredDictation {
                        id: format!("dict-{}", now_ms()),
                        text,
                        created_at: now_ms(),
                        duration_sec: Some(dur),
                        model: engine::models::dict_asr_id(),
                        lang: engine::store::get_setting("dict_lang").unwrap_or_default(),
                    };
                    let _ = engine::store::save_dictation(&entry);
                    let _ = app.emit("dictation:new", &entry);
                }
            }
            Err(e) => {
                let _ = app.emit("dictation:error", e);
            }
        }
        let _ = app.emit(
            "dictation:state",
            DictState {
                recording: false,
                processing: false,
            },
        );
        hide_overlay(&app);
        refresh_tray(&app);
    });
}

/// Эмуляция Cmd/Ctrl+V — вставка на активный курсор. ВАЖНО (macOS): вызывать только с
/// главного потока (enigo обращается к TSM/HIToolbox). На macOS нужен доступ Accessibility;
/// без него вызов молча ничего не сделает (текст всё равно останется в буфере обмена).
fn paste_to_cursor() {
    // Windows: честный VK-код через SendInput (enigo-Unicode не комбинируется с Ctrl).
    #[cfg(target_os = "windows")]
    {
        keys::paste_ctrl_v();
    }
    // macOS: Cmd+V через enigo (Unicode+Meta тут работает). Требует Accessibility.
    #[cfg(not(target_os = "windows"))]
    {
        use enigo::{Direction, Enigo, Key, Keyboard, Settings};
        let Ok(mut enigo) = Enigo::new(&Settings::default()) else {
            return;
        };
        #[cfg(target_os = "macos")]
        let modifier = Key::Meta;
        #[cfg(not(target_os = "macos"))]
        let modifier = Key::Control;
        let _ = enigo.key(modifier, Direction::Press);
        let _ = enigo.key(Key::Unicode('v'), Direction::Click);
        let _ = enigo.key(modifier, Direction::Release);
    }
}

// ── Оверлей-индикатор записи ──

fn show_overlay(app: &AppHandle) {
    // Создание/показ окна на macOS обязано идти с главного потока (хоткей-хендлер и
    // воркер транскрипции — не главные потоки).
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        use tauri::{WebviewUrl, WebviewWindowBuilder};
        if let Some(win) = app.get_webview_window("dict-overlay") {
            let _ = win.show();
            return;
        }
        let built = WebviewWindowBuilder::new(
            &app,
            "dict-overlay",
            WebviewUrl::App("index.html#/overlay".into()),
        )
        .title("")
        .inner_size(200.0, 64.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .build();
        if let Ok(win) = built {
            position_overlay(&win);
        }
    });
}

fn position_overlay(win: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = win.current_monitor().or_else(|_| win.primary_monitor()) {
        let size = monitor.size();
        if let Ok(win_size) = win.outer_size() {
            let x = (size.width as i32 - win_size.width as i32) / 2;
            // Верх экрана: отступ пропорционален высоте (как раньше был снизу).
            let y = size.height as i32 / 8;
            let _ = win.set_position(tauri::PhysicalPosition::new(x.max(0), y.max(0)));
        }
    }
}

fn hide_overlay(app: &AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(win) = app.get_webview_window("dict-overlay") {
            let _ = win.hide();
        }
    });
}

// ═══════════════════════ Разрешения macOS ═══════════════════════
// Для диктовки на macOS нужны два разрешения: «Мониторинг ввода» (глобальная клавиша,
// CGEventTap) и «Универсальный доступ» (авто-вставка, enigo). На Windows не требуются.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PermissionsStatus {
    needed: bool,           // нужны ли разрешения на этой ОС
    accessibility: bool,    // Универсальный доступ (авто-вставка)
    input_monitoring: bool, // Мониторинг ввода (глобальная клавиша)
}

#[cfg(target_os = "macos")]
mod macperms {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::CFString;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }
    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOHIDCheckAccess(request: u32) -> u32;
        fn IOHIDRequestAccess(request: u32) -> bool;
    }
    const LISTEN_EVENT: u32 = 1; // kIOHIDRequestTypeListenEvent
    const GRANTED: u32 = 0; // kIOHIDAccessTypeGranted

    pub fn accessibility() -> bool {
        unsafe { AXIsProcessTrusted() }
    }
    pub fn input_monitoring() -> bool {
        unsafe { IOHIDCheckAccess(LISTEN_EVENT) == GRANTED }
    }
    pub fn prompt_accessibility() {
        let key = CFString::from_static_string("AXTrustedCheckOptionPrompt");
        let val = CFBoolean::true_value();
        let dict = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), val.as_CFType())]);
        unsafe {
            AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef());
        }
    }
    pub fn prompt_input_monitoring() {
        unsafe {
            IOHIDRequestAccess(LISTEN_EVENT);
        }
    }
}

#[tauri::command]
fn permissions_status() -> PermissionsStatus {
    #[cfg(target_os = "macos")]
    {
        PermissionsStatus {
            needed: true,
            accessibility: macperms::accessibility(),
            input_monitoring: macperms::input_monitoring(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        PermissionsStatus {
            needed: false,
            accessibility: true,
            input_monitoring: true,
        }
    }
}

/// Запросить разрешение: показать системный промпт и открыть нужный раздел настроек.
#[tauri::command]
fn request_permission(kind: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        match kind.as_str() {
            "accessibility" => {
                macperms::prompt_accessibility();
                open_privacy_settings("Privacy_Accessibility");
            }
            "input-monitoring" => {
                macperms::prompt_input_monitoring();
                open_privacy_settings("Privacy_ListenEvent");
            }
            _ => return Err("неизвестное разрешение".into()),
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = kind;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn open_privacy_settings(anchor: &str) {
    let url = format!("x-apple.systempreferences:com.apple.preference.security?{anchor}");
    let _ = std::process::Command::new("open").arg(url).spawn();
}

// ═══════════════════════ MCP-сервер (команды) ═══════════════════════

fn mcp_port_setting() -> u16 {
    engine::store::get_setting("mcp_port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(8722)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct McpStatus {
    running: bool,
    port: u16,
    url: String,
    autostart: bool,
    has_token: bool,
    tools: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct McpConfig {
    enabled: bool,
    port: u16,
    token: String,
    autostart: bool,
}

fn mcp_status_inner() -> McpStatus {
    let running = mcp::is_running();
    let port = mcp::running_port().unwrap_or_else(mcp_port_setting);
    McpStatus {
        running,
        port,
        url: format!("http://127.0.0.1:{port}/mcp"),
        autostart: get_bool("mcp_autostart", false),
        has_token: engine::store::get_setting("mcp_token")
            .map(|t| !t.is_empty())
            .unwrap_or(false),
        tools: mcp::TOOL_NAMES.iter().map(|s| s.to_string()).collect(),
    }
}

#[tauri::command]
fn mcp_status() -> McpStatus {
    mcp_status_inner()
}

#[tauri::command]
fn mcp_config() -> McpConfig {
    McpConfig {
        enabled: mcp::is_running(),
        port: mcp_port_setting(),
        token: engine::store::get_setting("mcp_token").unwrap_or_default(),
        autostart: get_bool("mcp_autostart", false),
    }
}

fn mcp_token_opt() -> Option<String> {
    engine::store::get_setting("mcp_token").filter(|t| !t.is_empty())
}

#[tauri::command]
fn mcp_start(app: AppHandle) -> Result<McpStatus, String> {
    let actual = mcp::start(mcp_port_setting(), mcp_token_opt())?;
    let _ = engine::store::set_setting("mcp_port", &actual.to_string());
    refresh_tray(&app);
    Ok(mcp_status_inner())
}

#[tauri::command]
fn mcp_stop(app: AppHandle) -> McpStatus {
    mcp::stop();
    refresh_tray(&app);
    mcp_status_inner()
}

#[tauri::command]
fn set_mcp_config(app: AppHandle, config: McpConfig) -> Result<McpStatus, String> {
    engine::store::set_setting("mcp_port", &config.port.to_string())?;
    engine::store::set_setting("mcp_token", &config.token)?;
    engine::store::set_setting("mcp_autostart", if config.autostart { "1" } else { "0" })?;
    let token = if config.token.is_empty() {
        None
    } else {
        Some(config.token.clone())
    };
    if mcp::is_running() {
        // применяем новые настройки — перезапуск
        mcp::stop();
    }
    if config.enabled {
        mcp::start(config.port, token)?;
    }
    refresh_tray(&app);
    Ok(mcp_status_inner())
}

// ═══════════════════════ Трей ═══════════════════════

fn tray_status_item() -> &'static Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>> {
    static S: OnceLock<Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

fn tray_status_text() -> String {
    if engine::dictation::is_recording() {
        "● Идёт запись диктовки".into()
    } else if mcp::is_running() {
        format!("MCP-сервер: вкл :{}", mcp::running_port().unwrap_or(0))
    } else {
        "Готов".into()
    }
}

/// Обновить строку статуса в трее на главном потоке (muda требует main-thread).
fn refresh_tray(app: &AppHandle) {
    let _ = app.run_on_main_thread(|| {
        if let Some(item) = tray_status_item().lock().unwrap().as_ref() {
            let _ = item.set_text(tray_status_text());
        }
    });
}

fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

#[cfg(desktop)]
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    // Левый клик по иконке — показать окно. Правый клик — контекстное меню (статус + выход).
    let status = MenuItemBuilder::with_id("status", tray_status_text())
        .enabled(false)
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Выход").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&status)
        .separator()
        .item(&quit)
        .build()?;

    *tray_status_item().lock().unwrap() = Some(status);

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("SpeakAgent")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "quit" {
                really_quit().store(true, Ordering::Relaxed);
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    // Монохромная template-иконка (как у нативных приложений): чёрный силуэт + альфа,
    // macOS сам красит под светлую/тёмную строку меню. Фолбэк — иконка приложения.
    static TRAY_ICON: &[u8] = include_bytes!("../icons/tray-template.png");
    match tauri::image::Image::from_bytes(TRAY_ICON) {
        Ok(img) => {
            builder = builder.icon(img).icon_as_template(true);
        }
        Err(_) => {
            if let Some(icon) = app.default_window_icon().cloned() {
                builder = builder.icon(icon);
            }
        }
    }
    builder.build(app)?;
    Ok(())
}

/// Инициализация при старте: хоткей диктовки, автозапуск MCP, трей, сворачивание в трей.
fn setup_app(app: &tauri::App) {
    let handle = app.handle().clone();

    let spec = engine::store::get_setting("dict_keys").unwrap_or_else(|| DEFAULT_HOTKEY.into());
    set_trigger(&spec);
    start_key_listener(&handle);

    if get_bool("mcp_autostart", false) {
        let _ = mcp::start(mcp_port_setting(), mcp_token_opt());
    }

    #[cfg(desktop)]
    if let Err(e) = build_tray(app) {
        eprintln!("tray: {e}");
    }

    // Закрытие окна = сворачивание в трей (если включено и не настоящий выход).
    if let Some(win) = app.get_webview_window("main") {
        let w = win.clone();
        win.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if get_bool("close_to_tray", true) && !really_quit().load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
        });
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            apply_window_effects(app);
            setup_app(app);
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
            diagnostics,
            dictation_config,
            set_dictation_config,
            list_input_devices,
            dictation_recording,
            dictation_start,
            dictation_stop,
            list_dictations,
            delete_dictation,
            clear_dictations,
            permissions_status,
            request_permission,
            mcp_status,
            mcp_config,
            mcp_start,
            mcp_stop,
            set_mcp_config
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app, event| {
            // гасим sidecar llama-server и MCP-сервер вместе с приложением
            if matches!(event, tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }) {
                engine::llm::shutdown();
                mcp::stop();
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
