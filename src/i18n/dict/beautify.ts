const ru = {
  // Экран записи
  original: "Оригинал",
  beautified: "Обработанный",
  process: "Обработать текст",
  reprocess: "Обработать заново",
  processing: (done: number, total: number) =>
    total > 0 ? `Обрабатываю… ${done} из ${total}` : "Обрабатываю…",
  stop: "Стоп",
  emptyTitle: "Обработанный текст ещё не готов",
  emptyHint:
    "LLM аккуратно поправит орфографию, пунктуацию и очевидные ошибки распознавания, стараясь не менять смысл и формулировки. Оригинал всегда доступен по переключателю.",
  costNote:
    "Локально это может занять несколько минут (можно уйти на другие вкладки); через облачный ИИ — оплатится по токенам вашего провайдера.",
  retry: "Попробовать снова",
  needEngine:
    "Сначала подключите ИИ: локальную модель «Итогов» или облачный провайдер в Настройках.",
  // Настройки
  settingsTitle: "Обработка текста (украшатель)",
  settingsHint:
    "Опциональная LLM-очистка расшифровки: правит орфографию, пунктуацию и очевидные ослышки, стараясь не менять смысл. Выключено по умолчанию — на CPU это долго.",
  enableLabel: "Включить украшатель текста",
  enableHint: "Добавляет на экране записи переключатель «Оригинал | Обработанный».",
  autoLabel: "Обрабатывать автоматически после расшифровки",
  autoHint: "Осторожно: заметно удлиняет обработку каждого файла (выполняется, когда очередь пуста).",
};

type T = typeof ru;

const en: T = {
  original: "Original",
  beautified: "Cleaned up",
  process: "Clean up text",
  reprocess: "Clean up again",
  processing: (done, total) =>
    total > 0 ? `Processing… ${done} of ${total}` : "Processing…",
  stop: "Stop",
  emptyTitle: "The cleaned-up text isn't ready yet",
  emptyHint:
    "The LLM will carefully fix spelling, punctuation, and obvious recognition errors while trying not to change the meaning or the wording. The original stays available via the toggle.",
  costNote:
    "Locally this may take a few minutes (feel free to switch tabs); via a cloud AI it is billed by your provider's tokens.",
  retry: "Try again",
  needEngine:
    "Connect an AI first: a local summary model or a cloud provider in Settings.",
  settingsTitle: "Text cleanup (beautifier)",
  settingsHint:
    "Optional LLM cleanup of the transcript: fixes spelling, punctuation, and obvious mishears while trying not to change the meaning. Off by default — it is slow on CPU.",
  enableLabel: "Enable text cleanup",
  enableHint: "Adds an “Original | Cleaned up” toggle on the recording screen.",
  autoLabel: "Clean up automatically after transcription",
  autoHint: "Careful: noticeably lengthens each file's processing (runs when the queue is empty).",
};

export const beautify = { ru, en };
