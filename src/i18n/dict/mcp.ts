const ru = {
  title: "MCP-сервер",
  intro:
    "Локальный сервер Model Context Protocol. Любой ИИ/код-агент (Claude Code, Cursor, VS Code) может обращаться к движку SpeakAgent — распознавать файлы, делать диаризацию, протоколы и смотреть историю. Работает офлайн, только на этом компьютере.",

  running: "Сервер запущен",
  stopped: "Сервер остановлен",
  start: "Запустить",

  connectHeading: "Подключение агента",
  quickAdd: "Быстрое добавление в популярные клиенты:",
  claudeCodeHint: "Claude Code — выполните в терминале:",
  configHint: "Cursor / VS Code / Claude Desktop — добавьте в конфиг MCP:",
  bearerToken: "Bearer <ваш токен>",

  toolsHeading: "Инструменты",
  settingsHeading: "Настройки",
  port: "Порт",
  token: "Токен (необязательно)",
  tokenPlaceholder: "пусто — без авторизации (только localhost)",
  autostart: "Запускать сервер при старте приложения",
  saveApply: "Сохранить и применить",

  // Подсказки к инструментам MCP, ключ = id инструмента.
  tools: {
    status: "версия, активная модель, готовность",
    transcribe: "распознать файл → текст",
    diarize: "распознать с разделением по говорящим",
    protocol: "протокол/саммари встречи (LLM)",
    todo: "список задач из записи (LLM)",
    summarize: "итог по записи из истории",
    list_jobs: "список записей истории",
    get_transcript: "текст записи по id",
  },
};

type T = typeof ru;

const en: T = {
  title: "MCP Server",
  intro:
    "A local Model Context Protocol server. Any AI or coding agent (Claude Code, Cursor, VS Code) can reach the SpeakAgent engine — transcribe files, run diarization, build protocols, and browse history. Works offline, on this computer only.",

  running: "Server running",
  stopped: "Server stopped",
  start: "Start",

  connectHeading: "Connect an agent",
  quickAdd: "Quick add to popular clients:",
  claudeCodeHint: "Claude Code — run in the terminal:",
  configHint: "Cursor / VS Code / Claude Desktop — add to your MCP config:",
  bearerToken: "Bearer <your token>",

  toolsHeading: "Tools",
  settingsHeading: "Settings",
  port: "Port",
  token: "Token (optional)",
  tokenPlaceholder: "empty — no authorization (localhost only)",
  autostart: "Start the server when the app launches",
  saveApply: "Save & apply",

  tools: {
    status: "version, active model, readiness",
    transcribe: "transcribe a file → text",
    diarize: "transcribe with speaker separation",
    protocol: "meeting protocol/summary (LLM)",
    todo: "task list from a recording (LLM)",
    summarize: "summary of a recording from history",
    list_jobs: "list of history recordings",
    get_transcript: "recording text by id",
  },
};

export const mcp = { ru, en };
