//! Локальный MCP-сервер (Model Context Protocol) внутри приложения.
//!
//! Транспорт — Streamable HTTP на `127.0.0.1:<порт>`: любой код-агент (Claude Code,
//! Cursor, VS Code) подключается по URL `http://127.0.0.1:<порт>/mcp`. Протокол —
//! JSON-RPC 2.0; сервер stateless (отвечает `application/json`, без SSE-сессий), чего
//! достаточно для перечисленных клиентов. Инструменты — тонкая обёртка над `engine::*`
//! (тот же офлайн-движок, что и в UI): распознавание, диаризация, «Итоги», история.
//!
//! Жизненный цикл — как у llama-sidecar: синглтон, старт/стоп по команде из UI, гашение
//! на выходе приложения. Сервер крутится на своём tokio-рантайме в отдельном OS-потоке,
//! чтобы не мешать рантайму Tauri; порт биндим синхронно (ловим «занят» сразу).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::engine;

/// Имена экспортируемых инструментов (для UI-страницы MCP).
pub const TOOL_NAMES: &[&str] = &[
    "status",
    "transcribe",
    "diarize",
    "protocol",
    "todo",
    "summarize",
    "list_jobs",
    "get_transcript",
];

struct Running {
    port: u16,
    shutdown: Arc<tokio::sync::Notify>,
    thread: std::thread::JoinHandle<()>,
}

fn state() -> &'static Mutex<Option<Running>> {
    static S: OnceLock<Mutex<Option<Running>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(None))
}

/// Запущен ли сервер.
pub fn is_running() -> bool {
    state().lock().unwrap().is_some()
}

/// Порт запущенного сервера (если запущен).
pub fn running_port() -> Option<u16> {
    state().lock().unwrap().as_ref().map(|r| r.port)
}

struct AppState {
    token: Option<String>,
}

/// Запустить сервер на 127.0.0.1:port (0 → свободный порт). Возвращает реальный порт.
pub fn start(port: u16, token: Option<String>) -> Result<u16, String> {
    let mut g = state().lock().unwrap();
    if let Some(r) = g.as_ref() {
        return Ok(r.port);
    }
    let std_listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("port {port} is not available: {e}"))?;
    std_listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let actual = std_listener.local_addr().map(|a| a.port()).unwrap_or(port);

    let shutdown = Arc::new(tokio::sync::Notify::new());
    let sd = shutdown.clone();
    let auth = token.filter(|t| !t.is_empty());

    let thread = std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("mcp: не удалось создать рантайм: {e}");
                return;
            }
        };
        rt.block_on(async move {
            let listener = match tokio::net::TcpListener::from_std(std_listener) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("mcp: listener: {e}");
                    return;
                }
            };
            let app = router(Arc::new(AppState { token: auth }));
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move { sd.notified().await })
                .await;
        });
    });

    *g = Some(Running {
        port: actual,
        shutdown,
        thread,
    });
    Ok(actual)
}

/// Остановить сервер (если запущен).
pub fn stop() {
    if let Some(r) = state().lock().unwrap().take() {
        r.shutdown.notify_waiters();
        let _ = r.thread.join();
    }
}

fn router(st: Arc<AppState>) -> Router {
    Router::new()
        .route("/mcp", post(handle).get(handle_get))
        .route("/", get(|| async { "SpeakAgent MCP — POST /mcp (JSON-RPC 2.0)" }))
        .with_state(st)
}

/// GET /mcp — SSE-стрим сервер→клиент не поддерживаем (stateless). Клиенты это терпят.
async fn handle_get() -> Response {
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}

async fn handle(State(st): State<Arc<AppState>>, headers: HeaderMap, body: String) -> Response {
    if let Some(tok) = &st.token {
        let ok = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|h| h.strip_prefix("Bearer ").unwrap_or(h) == tok)
            .unwrap_or(false);
        if !ok {
            return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
        }
    }

    let req: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            return Json(json!({
                "jsonrpc": "2.0", "id": Value::Null,
                "error": { "code": -32700, "message": "parse error" }
            }))
            .into_response()
        }
    };

    // Пакет (массив) запросов — отвечаем массивом (пропуская нотификации).
    if let Some(arr) = req.as_array() {
        let mut out = Vec::new();
        for item in arr {
            if let Some(resp) = dispatch(item).await {
                out.push(resp);
            }
        }
        if out.is_empty() {
            return StatusCode::ACCEPTED.into_response();
        }
        return Json(Value::Array(out)).into_response();
    }

    match dispatch(&req).await {
        Some(resp) => Json(resp).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

/// Обработать один JSON-RPC запрос. None — нотификация (без ответа).
async fn dispatch(req: &Value) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    // Нотификация (нет id): initialized / cancelled / progress — просто подтверждаем.
    let id = id?;

    let result: Result<Value, (i64, String)> = match method {
        "initialize" => Ok(initialize_result(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_defs() })),
        "tools/call" => call_tool(&params).await,
        "resources/list" => Ok(json!({ "resources": [] })),
        "prompts/list" => Ok(json!({ "prompts": [] })),
        other => Err((-32601, format!("method not supported: {other}"))),
    };

    Some(match result {
        Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
        Err((code, msg)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": msg } })
        }
    })
}

fn initialize_result(params: &Value) -> Value {
    let ver = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("2025-06-18");
    json!({
        "protocolVersion": ver,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "speakagent", "version": env!("CARGO_PKG_VERSION") }
    })
}

// ───────────────────────── Инструменты ─────────────────────────

fn tool_defs() -> Vec<Value> {
    let src = |desc: &str| json!({ "type": "string", "description": desc });
    vec![
        json!({
            "name": "status",
            "description": "App status: version, active model, ASR/LLM readiness, data folder.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "transcribe",
            "description": "Transcribe an audio/video file to text (offline). The result is saved to history. Returns text and jobId.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": src("Absolute path to a local audio/video file"),
                    "language": { "type": "string", "description": "Language hint (ru|en), optional" }
                },
                "required": ["source"]
            }
        }),
        json!({
            "name": "diarize",
            "description": "Transcribe a file with speaker separation (SpeakerN [time]: text). Saved to history.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": src("Absolute path to a local audio/video file"),
                    "numSpeakers": { "type": "integer", "description": "Exact number of speakers (0/empty — auto)" },
                    "language": { "type": "string" }
                },
                "required": ["source"]
            }
        }),
        json!({
            "name": "protocol",
            "description": "Produce a meeting protocol/summary from a file or a history recording (local LLM).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": src("jobId from history OR a file path"),
                    "style": { "type": "string", "enum": ["summary", "business", "interview"], "description": "Style (default business)" }
                },
                "required": ["source"]
            }
        }),
        json!({
            "name": "todo",
            "description": "Extract a task list (checklist) from a file or a history recording (local LLM).",
            "inputSchema": {
                "type": "object",
                "properties": { "source": src("jobId from history OR a file path") },
                "required": ["source"]
            }
        }),
        json!({
            "name": "summarize",
            "description": "Generate a summary for an existing history recording.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "jobId": src("History recording ID (list_jobs)"),
                    "kind": { "type": "string", "enum": ["summary", "business", "interview", "todo"], "description": "Summary type" }
                },
                "required": ["jobId"]
            }
        }),
        json!({
            "name": "list_jobs",
            "description": "List of transcription history records (id, name, date, duration).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "get_transcript",
            "description": "Get the transcript text by history recording ID.",
            "inputSchema": {
                "type": "object",
                "properties": { "jobId": src("History recording ID") },
                "required": ["jobId"]
            }
        }),
    ]
}

async fn call_tool(params: &Value) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or((-32602i64, "tool name not specified".to_string()))?
        .to_string();
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let out = tokio::task::spawn_blocking(move || run_tool(&name, args))
        .await
        .map_err(|e| (-32603i64, e.to_string()))?;

    Ok(match out {
        Ok(v) => v,
        Err(msg) => error_result(msg),
    })
}

fn run_tool(name: &str, args: Value) -> Result<Value, String> {
    match name {
        "status" => tool_status(),
        "transcribe" => tool_transcribe(&args, false),
        "diarize" => tool_transcribe(&args, true),
        "protocol" => tool_protocol(&args),
        "todo" => tool_todo(&args),
        "summarize" => tool_summarize(&args),
        "list_jobs" => tool_list_jobs(),
        "get_transcript" => tool_get_transcript(&args),
        other => Err(format!("unknown tool: {other}")),
    }
}

fn tool_status() -> Result<Value, String> {
    let ready = engine::models::active_asr_files().is_some();
    let llm_ready = engine::llm::is_ready() || engine::llm::cloud_config().is_some();
    let structured = json!({
        "app": "SpeakAgent",
        "version": env!("CARGO_PKG_VERSION"),
        "asrReady": ready,
        "activeModel": engine::models::active_id(),
        "dictationModel": engine::models::dict_asr_id(),
        "llmReady": llm_ready,
        "dataDir": engine::store::data_dir().to_string_lossy(),
    });
    let text = format!(
        "SpeakAgent v{} — model: {}, ASR ready: {}, LLM ready: {}",
        env!("CARGO_PKG_VERSION"),
        engine::models::active_id(),
        ready,
        llm_ready
    );
    Ok(text_result(text, Some(structured)))
}

fn tool_transcribe(args: &Value, diarize: bool) -> Result<Value, String> {
    let source = arg_str(args, "source")?;
    let num = args.get("numSpeakers").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let (job_id, text, dur) = transcribe_file(&source, diarize, num)?;
    let structured = json!({ "text": text, "jobId": job_id, "durationSec": dur });
    Ok(text_result(text, Some(structured)))
}

fn tool_protocol(args: &Value) -> Result<Value, String> {
    let source = arg_str(args, "source")?;
    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("business");
    let kind = match style {
        "summary" => "summary",
        "interview" => "interview",
        _ => "business",
    };
    let job_id = resolve_source_to_job(&source)?;
    llm_generate_job(&job_id, kind)
}

fn tool_todo(args: &Value) -> Result<Value, String> {
    let source = arg_str(args, "source")?;
    let job_id = resolve_source_to_job(&source)?;
    llm_generate_job(&job_id, "todo")
}

fn tool_summarize(args: &Value) -> Result<Value, String> {
    let job_id = arg_str(args, "jobId")?;
    let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("summary");
    llm_generate_job(&job_id, kind)
}

fn tool_list_jobs() -> Result<Value, String> {
    let jobs = engine::store::list()?;
    let arr: Vec<Value> = jobs
        .iter()
        .map(|j| {
            json!({
                "id": j.id, "name": j.name, "createdAt": j.created_at,
                "durationSec": j.duration_sec, "diarize": j.diarize,
                "status": j.status, "chars": j.text.chars().count()
            })
        })
        .collect();
    let text = format!("{} records in history", arr.len());
    Ok(text_result(text, Some(json!({ "jobs": arr }))))
}

fn tool_get_transcript(args: &Value) -> Result<Value, String> {
    let job_id = arg_str(args, "jobId")?;
    let job = engine::store::get(&job_id).ok_or("recording not found in history")?;
    Ok(text_result(
        job.text.clone(),
        Some(json!({ "text": job.text, "name": job.name, "durationSec": job.duration_sec })),
    ))
}

// ───────────────────────── Общая логика ─────────────────────────

fn num_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| (n.get() as i32).clamp(1, 16))
        .unwrap_or(4)
}

/// Распознать файл (+ опц. диаризация), сохранить в историю. → (jobId, text, durSec).
fn transcribe_file(path: &str, diarize: bool, num_speakers: i32) -> Result<(String, String, f64), String> {
    if !std::path::Path::new(path).exists() {
        return Err(format!("file not found: {path}"));
    }
    let samples = engine::decode::decode_to_16k_mono(path)?;
    let audio_sec = samples.len() as f64 / 16000.0;
    let files = engine::models::active_asr_files()
        .ok_or("Recognition model is not installed. Open Settings and choose a model.")?;
    let asr = engine::asr::Asr::load(&files, num_threads())?;
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let vad = engine::models::vad();
    let words = asr.transcribe_words(&samples, vad.as_deref(), &cancel, |_, _, _| {});

    let text = if diarize {
        let (seg, emb) = engine::models::diarization()
            .ok_or("Diarization models are not installed. Download them in Settings.")?;
        let segs = engine::diarize::diarize(
            &seg,
            &emb,
            &samples,
            num_threads(),
            0.8,
            num_speakers.max(0),
        )?;
        engine::diarize::words_to_replicas(&words, &segs)
    } else {
        words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };

    let id = new_id();
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("MCP")
        .to_string();
    let job = engine::store::StoredJob {
        id: id.clone(),
        name,
        path: path.to_string(),
        diarize,
        status: "done".into(),
        text: text.clone(),
        error: String::new(),
        created_at: now_ms(),
        speakers: "{}".into(),
        duration_sec: Some(audio_sec),
    };
    let _ = engine::store::save(&job);
    Ok((id, text, audio_sec))
}

/// Источник → jobId. Существующий jobId возвращаем как есть; путь к файлу — распознаём.
fn resolve_source_to_job(source: &str) -> Result<String, String> {
    if engine::store::get(source).is_some() {
        return Ok(source.to_string());
    }
    if std::path::Path::new(source).exists() {
        let (id, _t, _d) = transcribe_file(source, false, 0)?;
        return Ok(id);
    }
    Err("source not found: provide a jobId from history or a path to an existing file".into())
}

fn llm_generate_job(job_id: &str, kind: &str) -> Result<Value, String> {
    let rkind = engine::llm::ResultKind::parse(kind)
        .ok_or("unknown summary type (summary|business|interview|todo)")?;
    if !engine::llm::is_ready() && engine::llm::cloud_config().is_none() {
        return Err(
            "Local LLM is not installed. Open Settings → Summaries and download the assistant (or configure a cloud provider)."
                .into(),
        );
    }
    let job = engine::store::get(job_id).ok_or("recording not found in history")?;
    if job.text.trim().is_empty() {
        return Err("recording has no text".into());
    }
    let transcript = engine::llm::prepare_transcript(&job.text, &job.speakers);
    let digest = engine::store::digest_for(job_id);
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let gen = engine::llm::generate(rkind, &transcript, digest.as_deref(), &cancel, |_| {})?;

    let now = now_ms();
    let model = engine::models::active_llm_id();
    if let Some(d) = &gen.digest {
        let _ = engine::store::save_result(&engine::store::JobResult {
            job_id: job_id.to_string(),
            kind: "digest".into(),
            text: d.clone(),
            model: model.clone(),
            created_at: now,
        });
    }
    let _ = engine::store::save_result(&engine::store::JobResult {
        job_id: job_id.to_string(),
        kind: rkind.as_str().into(),
        text: gen.text.clone(),
        model,
        created_at: now,
    });
    Ok(text_result(gen.text, None))
}

// ───────────────────────── Хелперы ─────────────────────────

fn arg_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing required parameter: {key}"))
}

fn text_result(text: String, structured: Option<Value>) -> Value {
    let mut r = json!({
        "content": [ { "type": "text", "text": text } ],
        "isError": false
    });
    if let Some(s) = structured {
        r["structuredContent"] = s;
    }
    r
}

fn error_result(msg: String) -> Value {
    json!({
        "content": [ { "type": "text", "text": msg } ],
        "isError": true
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn new_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("mcp-{}-{}", now_ms(), n)
}

#[cfg(test)]
mod tests {
    use super::*;

    // POST JSON-RPC (ureq без фичи "json" — шлём строкой). Ok(Value) | Err(HTTP-код).
    fn post(url: &str, body: &Value, token: Option<&str>) -> Result<Value, u16> {
        let mut req = ureq::post(url).set("Content-Type", "application/json");
        if let Some(t) = token {
            req = req.set("Authorization", &format!("Bearer {t}"));
        }
        match req.send_string(&body.to_string()) {
            Ok(resp) => {
                let s = resp.into_string().unwrap_or_default();
                Ok(serde_json::from_str(&s).unwrap_or(Value::Null))
            }
            Err(ureq::Error::Status(code, _)) => Err(code),
            Err(_) => Err(0),
        }
    }

    /// Поднимаем сервер по HTTP и проверяем JSON-RPC handshake, список инструментов
    /// и авторизацию по токену. Один тест (сервер — синглтон, тесты не параллелим).
    /// Только протокол — движок/БД не трогаем.
    #[test]
    fn http_protocol_and_auth() {
        // ── Без токена ──
        let port = start(0, None).expect("сервер должен подняться");
        assert!(is_running());
        let url = format!("http://127.0.0.1:{port}/mcp");

        let resp = post(
            &url,
            &json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2025-06-18" }
            }),
            None,
        )
        .expect("initialize");
        assert_eq!(resp["result"]["serverInfo"]["name"], "speakagent");
        assert_eq!(resp["result"]["protocolVersion"], "2025-06-18");

        let resp = post(&url, &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }), None)
            .expect("tools/list");
        let tools = resp["result"]["tools"].as_array().expect("массив инструментов");
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        for expected in ["transcribe", "diarize", "protocol", "list_jobs"] {
            assert!(names.contains(&expected), "нет инструмента {expected}");
        }

        let resp = post(&url, &json!({ "jsonrpc": "2.0", "id": 3, "method": "ping" }), None)
            .expect("ping");
        assert!(resp["result"].is_object());

        stop();
        assert!(!is_running());

        // ── С токеном ──
        let port = start(0, Some("secret123".into())).expect("сервер должен подняться");
        let url = format!("http://127.0.0.1:{port}/mcp");

        let denied = post(&url, &json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }), None);
        assert_eq!(denied, Err(401), "без токена ожидаем 401");

        let resp = post(
            &url,
            &json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }),
            Some("secret123"),
        )
        .expect("ping с токеном");
        assert!(resp["result"].is_object());

        stop();
    }
}
