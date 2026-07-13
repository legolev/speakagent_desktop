// «Итоги встречи»: вкладки над результатом (Текст · Саммари · Протокол · Задачи),
// панель одного артефакта (генерация локальным LLM), докачка помощника и экспорт.
// Общие атомы (Копировать/Скопировано!/Остановить) берём из common.
const ru = {
  // Вкладки над результатом.
  tabText: "Текст",
  tabSummary: "Саммари",
  tabProtocol: "Протокол",
  tabTodo: "Задачи",

  // Заголовки артефактов (для имён экспортных файлов).
  kindSummary: "Саммари",
  kindBusiness: "Протокол",
  kindInterview: "Протокол собеседования",
  kindTodo: "Задачи",

  // Стиль протокола.
  styleBusiness: "Деловая встреча",
  styleInterview: "Собеседование",

  // Состояния генерации.
  reading: (done: number, total: number) => `Читаю запись… ${done} из ${total}`,
  writing: "Пишу…",
  starting: "Запускаю помощника…",
  readingHint: "Длинная запись — сначала прочитаю её по частям.",
  writingHint:
    "Это занимает от минуты до нескольких минут — можно уйти на другие вкладки.",

  // Готовый результат.
  regenerate: "Составить заново",

  // Пустое состояние / ошибка.
  tryAgain: "Попробовать снова",
  compose: "Составить",
  etaLocal: (eta: string) =>
    `На этом компьютере — примерно ${eta}. Всё считается локально, без интернета.`,
  minRange: (lo: number, hi: number) => `${lo}–${hi} мин`,

  // Запасной путь: скопировать запрос для внешнего ИИ.
  copyPrompt: "Скопировать запрос для ИИ",
  copyPromptHint:
    "Быстрая альтернатива: вставьте скопированное в ChatGPT, Claude или другой ИИ-чат — он составит итог по вашей расшифровке.",

  // Первичная докачка помощника.
  helperNeeded:
    "Для итогов нужен помощник — он скачается один раз (~2,4 ГБ) и дальше работает полностью офлайн.",
  downloadHelper: "Скачать помощника",

  // Экспорт артефакта.
  download: "Скачать",
  exportTxt: "Текст (.txt)",
};

type T = typeof ru;
const en: T = {
  tabText: "Text",
  tabSummary: "Summary",
  tabProtocol: "Minutes",
  tabTodo: "Tasks",

  kindSummary: "Summary",
  kindBusiness: "Minutes",
  kindInterview: "Interview minutes",
  kindTodo: "Tasks",

  styleBusiness: "Business meeting",
  styleInterview: "Interview",

  reading: (done, total) => `Reading the recording… ${done} of ${total}`,
  writing: "Writing…",
  starting: "Starting the assistant…",
  readingHint: "It's a long recording — I'll read it in parts first.",
  writingHint: "This takes from a minute to a few — feel free to switch tabs.",

  regenerate: "Regenerate",

  tryAgain: "Try again",
  compose: "Compose",
  etaLocal: (eta) => `On this computer — about ${eta}. Everything runs locally, no internet.`,
  minRange: (lo, hi) => `${lo}–${hi} min`,

  copyPrompt: "Copy the request for an AI",
  copyPromptHint:
    "A quick alternative: paste it into ChatGPT, Claude, or another AI chat — it'll build the summary from your transcript.",

  helperNeeded:
    "Summaries need an assistant — it downloads once (~2.4 GB) and then works fully offline.",
  downloadHelper: "Download the assistant",

  download: "Download",
  exportTxt: "Text (.txt)",
};

export const meeting = { ru, en };
