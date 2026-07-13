import { invoke, Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { tr } from "../i18n";

export interface FileInfo {
  name: string;
  size_bytes: number;
  size_human: string;
}

export interface AppInfo {
  name: string;
  version: string;
}

export interface Progress {
  stage: "decoding" | "diarizing" | "transcribing" | "punctuating" | "done";
  done: number;
  total: number;
  partial: string;
  audioSec: number; // длительность записи, сек (>0 на финальном сообщении)
}

export const appInfo = () => invoke<AppInfo>("app_info");
export const fileInfo = (path: string) => invoke<FileInfo>("file_info", { path });
export const openDataDir = () => invoke<void>("open_data_dir");
/** Открыть внешний http(s)-адрес в системном браузере. */
export const openUrl = (url: string) => invoke<void>("open_url", { url });
/** Показать файл в системном менеджере (Finder/Explorer) с выделением. */
export const revealFile = (path: string) => invoke<void>("reveal_file", { path });
/** Стабильный неизменяемый ID этого устройства. */
export const deviceId = () => invoke<string>("device_id");
/** Служебная информация одним текстом (для баг-репорта). */
export const diagnostics = () => invoke<string>("diagnostics");

export interface SystemInfo {
  physicalCores: number;
  logicalCores: number;
  ramTotalGb: number;
  ramAvailableGb: number;
  /** RTF расшифровки без диаризации. */
  rtfPlain: number;
  /** RTF расшифровки с диаризацией (дороже, калибруется отдельно). */
  rtfDiar: number;
  /** Фиксированный оверхед, сек (декод + загрузка моделей). */
  overheadSec: number;
  measured: boolean;
  speed: "fast" | "medium" | "slow";
  /** Видеокарта (если найдена). */
  gpuName: string | null;
  gpuVramGb: number;
  /** Чем ускоряются «Итоги»: gpu | cpu. */
  llmAccel: "gpu" | "cpu";
}
export interface Usage {
  cpuPct: number;
  ramUsedPct: number;
}
export const systemInfo = () => invoke<SystemInfo>("system_info");
export const probeDuration = (path: string) => invoke<number | null>("probe_duration", { path });
export const resourceUsage = () => invoke<Usage>("resource_usage");
export const saveText = (content: string, path: string) =>
  invoke<void>("save_text", { content, path });

export interface PdfBlock {
  heading: string | null;
  time: string | null;
  body: string;
}
export const savePdf = (title: string, blocks: PdfBlock[], path: string) =>
  invoke<void>("save_pdf", { title, blocks, path });

export const transcribe = (
  path: string,
  diarize: boolean,
  jobId: string,
  onProgress: Channel<Progress>,
  numSpeakers?: number, // >0 — точное число говорящих; иначе авто
) =>
  invoke<string>("transcribe", {
    path,
    diarize,
    numSpeakers: numSpeakers && numSpeakers > 0 ? numSpeakers : null,
    jobId,
    onProgress,
  });

export const cancelTranscribe = (jobId: string) =>
  invoke<void>("cancel_transcribe", { jobId });

// ── История (SQLite на стороне Rust) ──
export interface StoredJob {
  id: string;
  name: string;
  path: string;
  diarize: boolean;
  status: "done" | "error";
  text: string;
  error: string;
  createdAt: number;
  speakers: string; // JSON {номер: имя}
  durationSec?: number | null; // длительность записи, сек
}

export const listJobs = () => invoke<StoredJob[]>("list_jobs");
export const saveJob = (job: StoredJob) => invoke<void>("save_job", { job });
export const deleteJob = (id: string) => invoke<void>("delete_job", { id });
export const clearJobs = () => invoke<void>("clear_jobs");

// ── Менеджер моделей ──
export interface ModelInfo {
  id: string;
  name: string;
  kind: "asr" | "diarization" | "tool" | "llm";
  lang: string;
  sizeMb: number;
  required: boolean;
  installed: boolean;
}

export interface DlProgress {
  done: number;
  total: number;
}

export const listModels = () => invoke<ModelInfo[]>("list_models");
export const downloadModel = (id: string, onProgress: Channel<DlProgress>) =>
  invoke<void>("download_model", { id, onProgress });
export const activeModel = () => invoke<string>("active_model");
export const setActiveModel = (id: string) => invoke<void>("set_active_model", { id });
/** Готова ли активная модель к работе (для онбординга). */
export const isReady = () => invoke<boolean>("is_ready");
/** Фоновая докачка инфраструктурных моделей (диаризация + ffmpeg) — молча. */
export const ensureCore = () => invoke<void>("ensure_core");

// ── «Итоги встречи» (локальный LLM) ──
export type ResultKind = "summary" | "business" | "interview" | "todo";

export interface LlmProgress {
  stage: "starting" | "reading" | "writing";
  done: number;
  total: number;
  partial: string;
}

export interface JobResult {
  jobId: string;
  kind: string;
  text: string;
  model: string;
  createdAt: number;
}

/** Готова ли фича «Итоги» (движок + модель установлены). */
export const llmReady = () => invoke<boolean>("llm_ready");
/** Докачать движок + активную модель итогов (с прогрессом). */
export const ensureLlm = (onProgress: Channel<DlProgress>) =>
  invoke<void>("ensure_llm", { onProgress });
/** Составить итог по записи истории; результат сохраняется на бэке. */
export const llmGenerate = (
  jobId: string,
  kind: ResultKind,
  onProgress: Channel<LlmProgress>,
) => invoke<string>("llm_generate", { jobId, kind, onProgress });
export const cancelLlm = (jobId: string) => invoke<void>("cancel_llm", { jobId });
export const listResults = (jobId: string) => invoke<JobResult[]>("list_results", { jobId });
/** Сохранить пользовательскую правку итога (чекбоксы задач и т.п.). */
export const saveResultText = (jobId: string, kind: string, text: string) =>
  invoke<void>("save_result_text", { jobId, kind, text });
/** Готовый запрос (промпт + расшифровка) для внешнего ИИ-чата. */
export const llmExportPrompt = (jobId: string, kind: ResultKind) =>
  invoke<string>("llm_export_prompt", { jobId, kind });
/** Короткое название записи (работает, только когда помощник уже в памяти). */
export const llmDisplayName = (jobId: string) => invoke<string>("llm_display_name", { jobId });
export const activeLlmModel = () => invoke<string>("active_llm_model");
export const setActiveLlmModel = (id: string) => invoke<void>("set_active_llm_model", { id });

// ── Украшатель (опциональная faithful-очистка расшифровки) ──
export interface BeautifyConfig {
  enabled: boolean;
  auto: boolean;
}
export const beautifyConfig = () => invoke<BeautifyConfig>("beautify_config");
export const setBeautifyConfig = (config: BeautifyConfig) =>
  invoke<void>("set_beautify_config", { config });
/** Прогнать расшифровку через LLM-украшатель; «обработанный» текст сохраняется на бэке. */
export const llmBeautify = (jobId: string, onProgress: Channel<LlmProgress>) =>
  invoke<string>("llm_beautify", { jobId, onProgress });
export const cancelBeautify = (jobId: string) => invoke<void>("cancel_beautify", { jobId });
/** «Обработанный» текст записи, если он уже строился (иначе null). */
export const beautifiedText = (jobId: string) =>
  invoke<string | null>("beautified_text", { jobId });
/** Удалить все артефакты записи (при «Расшифровать заново»). */
export const clearResults = (jobId: string) => invoke<void>("clear_results", { jobId });

// ── ИИ-провайдер: локальный движок ↔ облако (OpenAI-совместимое) ──
export type LlmBackend = "local" | "cloud";
export interface CloudConfig {
  url: string;
  model: string;
  key: string;
}
export const llmBackend = () => invoke<LlmBackend>("llm_backend");
export const setLlmBackend = (mode: LlmBackend) => invoke<void>("set_llm_backend", { mode });
export const cloudConfig = () => invoke<CloudConfig>("cloud_config");
export const setCloudConfig = (url: string, model: string, key: string) =>
  invoke<void>("set_cloud_config", { url, model, key });
/** Проверить связь с облачным провайдером — возвращает короткий ответ модели. */
export const testCloud = (url: string, model: string, key: string) =>
  invoke<string>("test_cloud", { url, model, key });

// ── Язык приложения (UI + промпты LLM) ──
export type UiLang = "ru" | "en";
/** Текущий язык приложения из настроек (по умолчанию "ru"). */
export const uiLanguage = () => invoke<UiLang>("ui_language");
/** Сохранить язык приложения (влияет и на язык вывода «Итогов»). */
export const setUiLanguage = (lang: UiLang) => invoke<void>("set_ui_language", { lang });

// ── Диктовка (push-to-talk) ──
export interface StoredDictation {
  id: string;
  text: string;
  createdAt: number;
  durationSec?: number | null;
  model: string;
  lang: string;
}

export interface DictationConfig {
  hotkey: string;
  mode: "hold" | "toggle";
  autopaste: boolean;
  sound: boolean;
  inputDevice: string; // "" — устройство по умолчанию
  model: string; // "" — как активная ASR-модель
  lang: string;
}

export const dictationConfig = () => invoke<DictationConfig>("dictation_config");
export const setDictationConfig = (config: DictationConfig) =>
  invoke<void>("set_dictation_config", { config });
export const listInputDevices = () => invoke<string[]>("list_input_devices");
export const dictationRecording = () => invoke<boolean>("dictation_recording");
export const dictationStart = () => invoke<void>("dictation_start");
export const dictationStop = () => invoke<void>("dictation_stop");
export const listDictations = () => invoke<StoredDictation[]>("list_dictations");
export const deleteDictation = (id: string) => invoke<void>("delete_dictation", { id });
export const clearDictations = () => invoke<void>("clear_dictations");

// Разрешения macOS (глобальная клавиша + авто-вставка).
export interface PermissionsStatus {
  needed: boolean;
  accessibility: boolean;
  inputMonitoring: boolean;
}
export const permissionsStatus = () => invoke<PermissionsStatus>("permissions_status");
export const requestPermission = (kind: "accessibility" | "input-monitoring") =>
  invoke<void>("request_permission", { kind });

// ── Локальный MCP-сервер ──
export interface McpStatus {
  running: boolean;
  port: number;
  url: string;
  autostart: boolean;
  hasToken: boolean;
  tools: string[];
}
export interface McpConfig {
  enabled: boolean;
  port: number;
  token: string;
  autostart: boolean;
}
export const mcpStatus = () => invoke<McpStatus>("mcp_status");
export const mcpConfig = () => invoke<McpConfig>("mcp_config");
export const mcpStart = () => invoke<McpStatus>("mcp_start");
export const mcpStop = () => invoke<McpStatus>("mcp_stop");
export const setMcpConfig = (config: McpConfig) =>
  invoke<McpStatus>("set_mcp_config", { config });

const MEDIA_EXTS = [
  "mp3", "m4a", "aac", "wav", "flac", "ogg", "opus", "oga",
  "mp4", "mov", "mkv", "webm", "avi", "m4v", "ts",
];

/** Нативный системный выбор файла. */
export async function pickAudioFile(): Promise<string | null> {
  const res = await open({
    multiple: false,
    directory: false,
    filters: [{ name: tr().common.pickerAudioVideo, extensions: MEDIA_EXTS }],
  });
  return typeof res === "string" ? res : null;
}

/** Нативный выбор одного или нескольких файлов (для очереди). */
export async function pickAudioFiles(): Promise<string[]> {
  const res = await open({
    multiple: true,
    directory: false,
    filters: [{ name: tr().common.pickerAudioVideo, extensions: MEDIA_EXTS }],
  });
  if (Array.isArray(res)) return res;
  return typeof res === "string" ? [res] : [];
}
