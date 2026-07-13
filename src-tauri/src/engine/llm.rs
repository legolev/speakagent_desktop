//! «Итоги встречи»: локальный LLM через sidecar `llama-server` (llama.cpp).
//! Сервер качается менеджером моделей как ffmpeg (не входит в сборку), живёт на
//! 127.0.0.1:<случайный порт>, говорим с ним по OpenAI-совместимому API через ureq.
//! Спавнится при первом действии, переиспользуется, гасится watchdog'ом при простое
//! (флага --sleep-idle-seconds в llama-server нет — проверено на b9957) и на выходе.
//!
//! Длинные расшифровки — map-reduce: чанки по границам строк → конспект каждого
//! (SUMMARIZE_CHUNK) → «дайджест» → финальный промпт по дайджесту. Дайджест
//! возвращается наружу и кэшируется в SQLite (job_results kind='digest'), поэтому
//! второй артефакт по той же записи стоит секунды, а не минуты.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::engine::{models, prompts};

/// Сообщение об отмене — по нему различаем «пользователь отменил» и реальную ошибку.
pub const CANCELLED: &str = "cancelled";

// Оценка токенов для русского: ~2.2 симв/токен; в лимитах считаем консервативно по 2.0.
const CHARS_PER_TOKEN: f32 = 2.0;
const CHUNK_CHARS: usize = 10_000; // map-шаг: влезает в 8k-контекст с запасом
const IDLE_SHUTDOWN_SECS: u64 = 300; // выгрузить модель из RAM после 5 мин простоя

/// Язык вывода LLM: RU по умолчанию, EN — если пользователь выбрал английский UI.
#[derive(Clone, Copy)]
enum Lang {
    Ru,
    En,
}

/// Язык вывода по настройке UI (`ui_lang`): "en" → English, иначе — русский.
fn out_lang() -> Lang {
    match crate::engine::store::get_setting("ui_lang").as_deref() {
        Some("en") => Lang::En,
        _ => Lang::Ru,
    }
}

struct Server {
    child: Child,
    port: u16,
    model_id: String,
}

fn server() -> &'static Mutex<Option<Server>> {
    static S: OnceLock<Mutex<Option<Server>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

fn last_use() -> &'static AtomicU64 {
    static T: OnceLock<AtomicU64> = OnceLock::new();
    T.get_or_init(|| AtomicU64::new(0))
}

fn touch() {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    last_use().store(now, Ordering::Relaxed);
}

/// Тип артефакта «Итогов». Каждому — свой промпт и потолок длины ответа.
#[derive(Clone, Copy, PartialEq)]
pub enum ResultKind {
    Summary,
    Business,
    Interview,
    Todo,
}

impl ResultKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "summary" => Some(Self::Summary),
            "business" => Some(Self::Business),
            "interview" => Some(Self::Interview),
            "todo" => Some(Self::Todo),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Business => "business",
            Self::Interview => "interview",
            Self::Todo => "todo",
        }
    }

    fn prompt(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::Summary, Lang::Ru) => prompts::SUMMARY,
            (Self::Summary, Lang::En) => prompts::SUMMARY_EN,
            (Self::Business, Lang::Ru) => prompts::BUSINESS,
            (Self::Business, Lang::En) => prompts::BUSINESS_EN,
            (Self::Interview, Lang::Ru) => prompts::INTERVIEW,
            (Self::Interview, Lang::En) => prompts::INTERVIEW_EN,
            (Self::Todo, Lang::Ru) => prompts::TODO,
            (Self::Todo, Lang::En) => prompts::TODO_EN,
        }
    }
}

/// Текст промпта для экспорта «отнесу в свой ИИ-чат» (см. lib.rs::llm_export_prompt).
pub fn prompt_text(kind: ResultKind) -> &'static str {
    let lang = out_lang();
    kind.prompt(lang)
}

impl ResultKind {

    /// Обязательный потолок генерации: локально это разница между минутой и «ушёл на 10 минут».
    fn max_tokens(self) -> u32 {
        match self {
            Self::Summary => 1536,
            Self::Business => 2048,
            Self::Interview => 2048,
            Self::Todo => 800,
        }
    }
}

/// Прогресс генерации для UI: reading — идём по фрагментам записи, writing — пишем текст.
pub struct Progress {
    pub stage: &'static str, // "starting" | "reading" | "writing"
    pub done: u32,
    pub total: u32,
    pub partial: String,
}

/// Результат генерации; digest возвращается, если строился map-reduce (кэшировать!).
pub struct GenOutput {
    pub text: String,
    pub digest: Option<String>,
}

/// Готова ли фича: облако с токеном ИЛИ локальный движок + активная модель.
pub fn is_ready() -> bool {
    cloud_config().is_some() || models::llm_files().is_some()
}

/// Настройки облачного ИИ-провайдера, если он выбран и токен задан.
/// Возвращает (base_url, model, api_key). URL/модель — с дефолтами (OpenRouter / gpt-4o-mini).
pub fn cloud_config() -> Option<(String, String, String)> {
    use crate::engine::store::get_setting;
    if get_setting("llm_backend").as_deref() != Some("cloud") {
        return None;
    }
    let key = get_setting("cloud_key").filter(|k| !k.trim().is_empty())?;
    let url = get_setting("cloud_url")
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| "https://openrouter.ai/api/v1".into());
    let model = get_setting("cloud_model")
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| "openai/gpt-4o-mini".into());
    Some((url, model, key))
}

/// Проверка связи с облачным провайдером — мини-запрос. Ok(ответ) или понятная ошибка.
pub fn cloud_test(url: &str, model: &str, key: &str) -> Result<String, String> {
    if key.trim().is_empty() {
        return Err("provide a token".into());
    }
    let cancel = AtomicBool::new(false);
    let reply = cloud_chat(
        url,
        model,
        key,
        "Ты проверяешь связь. Ответь коротко.",
        "Ответь одним словом: ок",
        0.4,
        8,
        &cancel,
    )?;
    Ok(reply.trim().chars().take(40).collect())
}

/// Один запрос к облачному OpenAI-совместимому провайдеру (без стрима — облако быстрое).
fn cloud_chat(
    base_url: &str,
    model: &str,
    key: &str,
    system: &str,
    user: &str,
    temperature: f32,
    max_tokens: u32,
    cancel: &AtomicBool,
) -> Result<String, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err(CANCELLED.into());
    }
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": temperature,
        "max_tokens": max_tokens,
    });
    let resp = ureq::post(&endpoint)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {key}"))
        .timeout(Duration::from_secs(180))
        .send_string(&body.to_string());
    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let detail = r.into_string().unwrap_or_default();
            let hint = match code {
                401 | 403 => " — check the token",
                402 => " — insufficient funds with the provider",
                404 => " — check the address or the model name",
                _ => "",
            };
            return Err(format!("cloud provider: HTTP {code}{hint}. {}", truncate_chars(&detail, 300)));
        }
        Err(e) => return Err(format!("cloud provider: {e}")),
    };
    let raw = resp.into_string().map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let text = v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
    if text.trim().is_empty() {
        return Err("the cloud returned an empty response".into());
    }
    Ok(clean_output(&text))
}

/// Сервер уже поднят (модель в памяти) — дешёвые доп. запросы вроде авто-названия.
pub fn is_warm() -> bool {
    server().lock().unwrap().is_some()
}

/// Контекст: на GPU — по видеопамяти, на CPU — по RAM (KV-кэш дорог на слабых машинах).
fn ctx_size(gpu: Option<&crate::engine::gpu::GpuInfo>) -> u32 {
    if let Some(g) = gpu {
        return if g.vram_gb() >= 8.0 { 16384 } else { 8192 };
    }
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    if sys.total_memory() as f64 / 1e9 >= 12.0 {
        16384
    } else {
        8192
    }
}

fn threads() -> usize {
    sysinfo::System::new().physical_core_count().unwrap_or(4).max(1)
}

/// chat_template_kwargs для конкретных моделей (thinking-модели глушим).
fn chat_kwargs(model_id: &str) -> Option<serde_json::Value> {
    match model_id {
        "llm-qwen3-17b" => Some(serde_json::json!({"enable_thinking": false})),
        _ => None,
    }
}

fn health(port: u16) -> bool {
    ureq::get(&format!("http://127.0.0.1:{port}/health"))
        .timeout(Duration::from_millis(700))
        .call()
        .is_ok()
}

/// Запустить (или переиспользовать) llama-server с активной моделью. Возвращает порт.
pub fn ensure_server() -> Result<u16, String> {
    let (bin, gguf) =
        models::llm_files().ok_or("The summary model is not installed. Download it in Settings.")?;
    let active = models::active_llm_id();
    touch();

    let mut guard = server().lock().unwrap();
    if let Some(s) = guard.as_mut() {
        if s.model_id == active && health(s.port) {
            return Ok(s.port);
        }
        let _ = s.child.kill(); // сменилась модель или сервер умер — перезапуск
        *guard = None;
    }

    // свободный порт: биндим :0, читаем номер, отпускаем
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map_err(|e| format!("no free port: {e}"))?
        .port();

    // stderr сервера — в лог-файл (диагностика, если не стартует)
    let log = std::fs::File::create(crate::engine::store::data_dir().join("llama-server.log"))
        .map_err(|e| e.to_string())?;

    // GPU-выгрузка: дискретная карта с достаточной VRAM → авто-оффлоад (в b9957
    // -ngl auto сам распределяет слои с запасом VRAM); иначе жёстко CPU.
    let gpu = crate::engine::gpu::should_offload();

    let mut cmd = Command::new(&bin);
    cmd.args([
        "-m",
        &gguf,
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
        "-c",
        &ctx_size(gpu.as_ref()).to_string(),
        "-t",
        &threads().to_string(),
        "--cache-type-k",
        "q8_0",
        "--no-webui",
    ]);
    if gpu.is_none() {
        // iGPU/нет Vulkan/мало VRAM: гарантируем CPU даже с Vulkan-сборкой
        cmd.args(["--device", "none"]);
    }
    cmd
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::from(log));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start the summary helper: {e}"))?;

    // ждём готовности (большой GGUF с HDD грузится долго)
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if health(port) {
            break;
        }
        if let Ok(Some(status)) = child.try_wait() {
            return Err(spawn_error(status));
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            return Err("the summary helper did not respond within 2 minutes".into());
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    *guard = Some(Server {
        child,
        port,
        model_id: active,
    });
    spawn_idle_watchdog();
    Ok(port)
}

/// Ранний выход процесса → понятная ошибка (в т.ч. отсутствие MSVC-рантайма на Windows).
fn spawn_error(status: std::process::ExitStatus) -> String {
    #[cfg(windows)]
    if status.code() == Some(-1073741515i32) {
        // 0xC0000135 STATUS_DLL_NOT_FOUND
        return "A required Windows system component is missing (Microsoft Visual C++ Redistributable). \
                Install it from here and try again: https://aka.ms/vs/17/release/vc_redist.x64.exe"
            .into();
    }
    format!("the summary helper failed to start ({status}). Details: llama-server.log in the data folder")
}

/// Раз в минуту проверяем простой; после 5 минут без запросов выгружаем модель из RAM.
fn spawn_idle_watchdog() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        std::thread::spawn(|| loop {
            std::thread::sleep(Duration::from_secs(60));
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let idle = now.saturating_sub(last_use().load(Ordering::Relaxed));
            if idle >= IDLE_SHUTDOWN_SECS {
                shutdown();
            }
        });
    });
}

/// Остановить сервер (выход из приложения, смена модели, простой).
pub fn shutdown() {
    if let Some(mut s) = server().lock().unwrap().take() {
        let _ = s.child.kill();
    }
}

/// Потоковый чат-запрос (SSE). Токены — в `on_token`; отмена — между токенами
/// (drop соединения абортит слот на сервере).
fn chat_stream(
    port: u16,
    system: &str,
    user: &str,
    temperature: f32,
    max_tokens: u32,
    cancel: &AtomicBool,
    mut on_token: impl FnMut(&str),
) -> Result<String, String> {
    touch();
    let mut body = serde_json::json!({
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        // температуру задаёт вызывающий: саммари — 0.7 (низкие зацикливают 4B),
        // украшатель — ниже (задача копирующая, зацикливания нет).
        "temperature": temperature,
        "top_p": 0.8,
        "max_tokens": max_tokens,
        "stream": true,
    });
    if let Some(kw) = chat_kwargs(&models::active_llm_id()) {
        body["chat_template_kwargs"] = kw;
    }

    let resp = ureq::post(&format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .set("Content-Type", "application/json")
        .timeout(Duration::from_secs(30 * 60)) // CPU-генерация бывает долгой
        .send_string(&body.to_string())
        .map_err(|e| format!("summary helper: {e}"))?;

    let mut out = String::new();
    let reader = BufReader::new(resp.into_reader());
    for line in reader.lines() {
        if cancel.load(Ordering::Relaxed) {
            return Err(CANCELLED.into()); // drop reader → сервер абортит генерацию
        }
        let line = line.map_err(|e| e.to_string())?;
        let Some(data) = line.strip_prefix("data: ") else { continue };
        if data == "[DONE]" {
            break;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
            if let Some(tok) = v["choices"][0]["delta"]["content"].as_str() {
                out.push_str(tok);
                on_token(tok);
            }
        }
    }
    touch();
    Ok(clean_output(&out))
}

/// chat_stream с двумя повторами на транспортных ошибках (отмену не ретраим).
fn chat_retry(
    port: u16,
    system: &str,
    user: &str,
    temperature: f32,
    max_tokens: u32,
    cancel: &AtomicBool,
    mut on_token: impl FnMut(&str),
) -> Result<String, String> {
    let mut last = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_secs(2));
            let _ = ensure_server(); // сервер мог упасть — поднимем
        }
        match chat_stream(port, system, user, temperature, max_tokens, cancel, &mut on_token) {
            Ok(s) => return Ok(s),
            Err(e) if e == CANCELLED => return Err(e),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// Сколько символов расшифровки влезает в один проход для данного промпта.
fn single_pass_limit(kind: ResultKind, lang: Lang) -> usize {
    let ctx = ctx_size(crate::engine::gpu::should_offload().as_ref()) as f32;
    let prompt_tokens = kind.prompt(lang).chars().count() as f32 / CHARS_PER_TOKEN;
    let budget = ctx - kind.max_tokens() as f32 - prompt_tokens - 256.0; // 256 — сервисный запас
    (budget.max(1024.0) * CHARS_PER_TOKEN) as usize
}

/// Разбивка по границам строк (реплик); сверхдлинную строку режем по символам.
fn split_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let push_cur = |cur: &mut String, out: &mut Vec<String>| {
        if !cur.trim().is_empty() {
            out.push(std::mem::take(cur));
        } else {
            cur.clear();
        }
    };
    for line in text.lines() {
        if line.chars().count() > max_chars {
            push_cur(&mut cur, &mut out);
            let chars: Vec<char> = line.chars().collect();
            for piece in chars.chunks(max_chars) {
                out.push(piece.iter().collect());
            }
            continue;
        }
        if !cur.is_empty() && cur.chars().count() + line.chars().count() + 1 > max_chars {
            push_cur(&mut cur, &mut out);
        }
        if !cur.is_empty() {
            cur.push('\n');
        }
        cur.push_str(line);
    }
    push_cur(&mut cur, &mut out);
    out
}

/// UTF-8-безопасное усечение по числу символов.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect()
}

/// Составить артефакт по расшифровке.
/// `digest` — кэшированный дайджест прошлого map-reduce по этой записи (если был);
/// в ответе `digest` заполнен, если map-reduce строился сейчас — кэшируйте его.
pub fn generate(
    kind: ResultKind,
    transcript: &str,
    digest: Option<&str>,
    cancel: &AtomicBool,
    mut on: impl FnMut(Progress),
) -> Result<GenOutput, String> {
    on(Progress { stage: "starting", done: 0, total: 0, partial: String::new() });
    let lang = out_lang();

    // Облачный провайдер (если выбран): один запрос — контекст у облака большой,
    // map-reduce не нужен; расшифровку слегка ограничиваем, чтобы не раздувать стоимость.
    if let Some((url, model, key)) = cloud_config() {
        on(Progress { stage: "writing", done: 0, total: 0, partial: String::new() });
        let input = truncate_chars(transcript, 240_000);
        let text = cloud_chat(&url, &model, &key, kind.prompt(lang), &input, 0.4, kind.max_tokens() + 512, cancel)?;
        return Ok(GenOutput { text, digest: None });
    }

    let port = ensure_server()?;
    let limit = single_pass_limit(kind, lang);

    // 1) короткая запись — один проход по самой расшифровке
    if transcript.chars().count() <= limit {
        let text = final_call(port, kind, transcript, cancel, lang, &mut on)?;
        return Ok(GenOutput { text, digest: None });
    }

    // 2) длинная, но дайджест уже есть — финальный проход по нему
    if let Some(d) = digest.filter(|d| !d.trim().is_empty()) {
        let input = truncate_chars(d, limit);
        let text = final_call(port, kind, &input, cancel, lang, &mut on)?;
        return Ok(GenOutput { text, digest: None });
    }

    // 3) map-reduce: конспектируем фрагменты → дайджест → финальный проход
    let chunk_prompt = match lang {
        Lang::Ru => prompts::SUMMARIZE_CHUNK,
        Lang::En => prompts::SUMMARIZE_CHUNK_EN,
    };
    let chunks = split_chunks(transcript, CHUNK_CHARS);
    let total = chunks.len() as u32;
    let mut parts: Vec<String> = Vec::with_capacity(chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(CANCELLED.into());
        }
        on(Progress { stage: "reading", done: i as u32, total, partial: String::new() });
        match chat_retry(port, chunk_prompt, chunk, 0.7, 700, cancel, |_| {}) {
            Ok(s) => parts.push(s),
            Err(e) if e == CANCELLED => return Err(e),
            Err(_) => parts.push(truncate_chars(chunk, 1400)), // фолбэк облака: сырой кусок
        }
    }
    on(Progress { stage: "reading", done: total, total, partial: String::new() });

    let built = parts.join("\n\n");
    let input = truncate_chars(&built, limit); // одноуровневый reduce, как в облаке
    let text = final_call(port, kind, &input, cancel, lang, &mut on)?;
    Ok(GenOutput { text, digest: Some(built) })
}

/// Финальный вызов с потоковым выводом в Progress.
fn final_call(
    port: u16,
    kind: ResultKind,
    input: &str,
    cancel: &AtomicBool,
    lang: Lang,
    on: &mut impl FnMut(Progress),
) -> Result<String, String> {
    let mut acc = String::new();
    chat_retry(port, kind.prompt(lang), input, 0.7, kind.max_tokens(), cancel, |tok| {
        acc.push_str(tok);
        on(Progress { stage: "writing", done: 0, total: 0, partial: acc.clone() });
    })
}

// ─────────────────────────── Украшатель (faithful cleanup) ───────────────────────────

/// Размер батча/куска украшателя (символы). Меньше, чем у саммари: выход украшателя
/// ≈ длине входа и должен уместиться в потолок max_tokens.
const BEAUTIFY_CHUNK_CHARS: usize = 3500;

/// Потолок ответа украшателя для входа в `input_chars` символов (выход ≈ вход).
fn beautify_max_tokens(input_chars: usize) -> u32 {
    ((input_chars as f32 / CHARS_PER_TOKEN * 1.4).ceil() as u32).clamp(64, 3072)
}

/// Снять ведущий номер «N. » / «N) » / «N:» / «N\t» из строки ответа (иначе — вернуть как есть).
fn strip_num_prefix(s: &str) -> &str {
    let t = s.trim_start();
    let ndigits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if ndigits == 0 {
        return s.trim();
    }
    let after = &t[ndigits..];
    for sep in [". ", ".", ") ", ")", ": ", ":", "\t", " - ", "- "] {
        if let Some(rest) = after.strip_prefix(sep) {
            return rest.trim_start();
        }
    }
    s.trim()
}

/// Разбить плоский текст на куски ≤ max_chars по границам слов (без разрыва слов).
fn split_words(text: &str, max_chars: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + word.chars().count() + 1 > max_chars {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Один запрос украшателя к активному движку (облако или локальный llama-server) на низкой
/// температуре — задача копирующая, детерминизм важнее «креатива».
fn beautify_call(
    cloud: Option<&(String, String, String)>,
    port: Option<u16>,
    system: &str,
    user: &str,
    max_tokens: u32,
    cancel: &AtomicBool,
) -> Result<String, String> {
    match cloud {
        Some((url, model, key)) => {
            cloud_chat(url, model, key, system, user, 0.2, max_tokens, cancel)
        }
        None => {
            let port = port.ok_or("local engine not started")?;
            chat_retry(port, system, user, 0.2, max_tokens, cancel, |_| {})
        }
    }
}

/// Опциональная faithful-очистка расшифровки выбранной LLM: правит орфографию, пунктуацию
/// и очевидные ослышки, НЕ меняя смысла и дословности. Диаризованный текст обрабатывается
/// по-реплично (префиксы «Speaker N [ts]: » и таймкоды сохраняются побайтово), плоский —
/// кусками. Выход имеет ту же форму, что и вход. Правка недеструктивна: при расхождении
/// числа строк или неправдоподобной длине реплика/кусок откатывается к оригиналу.
pub fn beautify(
    raw_text: &str,
    cancel: &AtomicBool,
    mut on: impl FnMut(Progress),
) -> Result<String, String> {
    on(Progress { stage: "starting", done: 0, total: 0, partial: String::new() });
    let lang = out_lang();
    let cloud = cloud_config();
    // локальный движок поднимаем один раз (порт переиспользуется всеми батчами)
    let port = if cloud.is_none() { Some(ensure_server()?) } else { None };
    let is_diarized = raw_text.lines().any(|l| l.trim_start().starts_with("Speaker"));
    if is_diarized {
        beautify_diarized(raw_text, lang, cloud.as_ref(), port, cancel, &mut on)
    } else {
        beautify_plain(raw_text, lang, cloud.as_ref(), port, cancel, &mut on)
    }
}

/// Диаризованный путь: чистим тела реплик батчами, структуру «SpeakerN [ts]:» не трогаем.
fn beautify_diarized(
    raw_text: &str,
    lang: Lang,
    cloud: Option<&(String, String, String)>,
    port: Option<u16>,
    cancel: &AtomicBool,
    on: &mut impl FnMut(Progress),
) -> Result<String, String> {
    let system = match lang {
        Lang::Ru => prompts::BEAUTIFY_TURNS,
        Lang::En => prompts::BEAUTIFY_TURNS_EN,
    };

    // Строка → (префикс, тело). Реплика: тело непусто. Прочее (пустые/непарсимые) — passthrough.
    struct Ln {
        prefix: String,
        body: String,
    }
    let mut lines: Vec<Ln> = Vec::new();
    for raw in raw_text.lines() {
        let mut parsed: Option<Ln> = None;
        if raw.trim_start().starts_with("Speaker") {
            if let Some(cut) = raw.find("]:") {
                let body = raw[cut + 2..].trim_start();
                if !body.is_empty() {
                    let plen = raw.len() - body.len(); // тело — суффикс строки
                    parsed = Some(Ln {
                        prefix: raw[..plen].to_string(),
                        body: body.to_string(),
                    });
                }
            }
        }
        lines.push(parsed.unwrap_or_else(|| Ln {
            prefix: raw.to_string(),
            body: String::new(),
        }));
    }

    let turn_idx: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.body.is_empty())
        .map(|(i, _)| i)
        .collect();
    if turn_idx.is_empty() {
        return Ok(raw_text.to_string());
    }

    // Группируем реплики в батчи по бюджету символов.
    let mut batches: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut cur_chars = 0usize;
    for &i in &turn_idx {
        let c = lines[i].body.chars().count() + 8; // +номер/перевод строки
        if !cur.is_empty() && cur_chars + c > BEAUTIFY_CHUNK_CHARS {
            batches.push(std::mem::take(&mut cur));
            cur_chars = 0;
        }
        cur.push(i);
        cur_chars += c;
    }
    if !cur.is_empty() {
        batches.push(cur);
    }

    let total = batches.len() as u32;
    for (bi, batch) in batches.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(CANCELLED.into());
        }
        on(Progress { stage: "reading", done: bi as u32, total, partial: String::new() });

        let user = batch
            .iter()
            .enumerate()
            .map(|(k, &i)| format!("{}. {}", k + 1, lines[i].body))
            .collect::<Vec<_>>()
            .join("\n");
        let mt = beautify_max_tokens(user.chars().count());
        let reply = beautify_call(cloud, port, system, &user, mt, cancel)?;

        // Снять номера, отфильтровать пустые, сопоставить по позиции.
        let got: Vec<String> = reply
            .lines()
            .map(|l| strip_num_prefix(l).to_string())
            .filter(|l| !l.trim().is_empty())
            .collect();
        // Число строк должно совпасть — иначе модель склеила/выкинула реплики → откат батча.
        if got.len() == batch.len() {
            for (k, &i) in batch.iter().enumerate() {
                let orig_len = lines[i].body.chars().count().max(1);
                let ratio = got[k].chars().count() as f32 / orig_len as f32;
                if (0.5..=2.0).contains(&ratio) {
                    lines[i].body = got[k].trim().to_string();
                }
            }
        }
    }
    on(Progress { stage: "reading", done: total, total, partial: String::new() });

    Ok(lines
        .iter()
        .map(|l| {
            if l.body.is_empty() {
                l.prefix.clone()
            } else {
                format!("{}{}", l.prefix, l.body)
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Плоский путь: недиаризованный текст чистим кусками по границам слов, склеиваем.
fn beautify_plain(
    raw_text: &str,
    lang: Lang,
    cloud: Option<&(String, String, String)>,
    port: Option<u16>,
    cancel: &AtomicBool,
    on: &mut impl FnMut(Progress),
) -> Result<String, String> {
    let system = match lang {
        Lang::Ru => prompts::BEAUTIFY_PLAIN,
        Lang::En => prompts::BEAUTIFY_PLAIN_EN,
    };
    let chunks = split_words(raw_text, BEAUTIFY_CHUNK_CHARS);
    if chunks.is_empty() {
        return Ok(raw_text.to_string());
    }
    let total = chunks.len() as u32;
    let mut out: Vec<String> = Vec::with_capacity(chunks.len());
    for (ci, chunk) in chunks.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(CANCELLED.into());
        }
        on(Progress { stage: "reading", done: ci as u32, total, partial: String::new() });
        let mt = beautify_max_tokens(chunk.chars().count());
        let cleaned = beautify_call(cloud, port, system, chunk, mt, cancel)?;
        let ratio = cleaned.chars().count() as f32 / chunk.chars().count().max(1) as f32;
        if (0.5..=2.0).contains(&ratio) && !cleaned.trim().is_empty() {
            out.push(cleaned.trim().to_string());
        } else {
            out.push(chunk.clone());
        }
    }
    on(Progress { stage: "reading", done: total, total, partial: String::new() });
    Ok(out.join(" "))
}

/// Короткое название записи по началу расшифровки (авто-переименование в истории).
pub fn display_name(transcript: &str, cancel: &AtomicBool) -> Result<String, String> {
    let lang = out_lang();
    let name_prompt = match lang {
        Lang::Ru => prompts::DISPLAY_NAME,
        Lang::En => prompts::DISPLAY_NAME_EN,
    };
    let port = ensure_server()?;
    let input = truncate_chars(transcript, 2000);
    let name = chat_retry(port, name_prompt, &input, 0.7, 32, cancel, |_| {})?;
    Ok(name.trim().trim_matches('"').trim_matches('«').trim_matches('»').to_string())
}

/// Подстановка пользовательских имён спикеров в расшифровку перед отправкой в LLM:
/// «Speaker2 [0:00:13]: …» → «Иван [0:00:13]: …». Протокол скажет «Иван», не «Speaker 2».
pub fn prepare_transcript(text: &str, names_json: &str) -> String {
    let names: std::collections::HashMap<String, String> =
        serde_json::from_str(names_json).unwrap_or_default();
    if names.is_empty() {
        return text.to_string();
    }
    text.lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("Speaker") {
                if let Some((num, tail)) = rest.split_once(' ') {
                    if let Some(name) = names.get(num).filter(|n| !n.trim().is_empty()) {
                        return format!("{} {tail}", name.trim());
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Чистка вывода: <think>-блоки (thinking-модели) и обёртка ```…``` целиком вокруг ответа.
fn clean_output(s: &str) -> String {
    let mut t = s.trim().to_string();
    // <think>…</think> в начале ответа
    if let Some(start) = t.find("<think>") {
        if let Some(end) = t.find("</think>") {
            if start < end {
                t = format!("{}{}", &t[..start], &t[end + "</think>".len()..])
                    .trim()
                    .to_string();
            }
        }
    }
    // ответ целиком в код-заборе: ```markdown\n…\n```
    if t.starts_with("```") {
        if let Some(first_nl) = t.find('\n') {
            if let Some(last_fence) = t.rfind("```") {
                if last_fence > first_nl {
                    t = t[first_nl + 1..last_fence].trim().to_string();
                }
            }
        }
    }
    t
}
