// Общие атомы, переиспользуемые многими экранами. Каждый фрагмент словаря сам
// гарантирует паритет ru/en (en: <Dict> = typeof ru).
const ru = {
  // Локаль для дат/чисел (Intl).
  locale: "ru-RU",

  // Кнопки/действия.
  cancel: "Отмена",
  delete: "Удалить",
  copy: "Копировать",
  copied: "Скопировано",
  copiedBang: "Скопировано!",
  save: "Сохранить",
  saved: "Сохранено",
  stop: "Остановить",
  back: "Назад",
  clear: "Очистить",

  // Метки состояний.
  notDownloaded: "не скачана",
  clickToRename: "Нажмите, чтобы переименовать",
  auto: "Авто",

  // Параметрические.
  noResultsFor: (q: string) => `Ничего не найдено по запросу «${q}».`,
  speaker: (n: number) => `Спикер ${n}`,
  cores: (n: number) => `${n} ядер`,
  ramGb: (gb: number | string) => `${gb} ГБ ОЗУ`,
  mb: (n: number) => `${n} МБ`,

  // Метки фильтров системных диалогов выбора/сохранения файлов.
  pickerAudioVideo: "Аудио и видео",
  pickerText: "Текст",
};

export type CommonDict = typeof ru;

const en: CommonDict = {
  locale: "en-US",

  cancel: "Cancel",
  delete: "Delete",
  copy: "Copy",
  copied: "Copied",
  copiedBang: "Copied!",
  save: "Save",
  saved: "Saved",
  stop: "Stop",
  back: "Back",
  clear: "Clear",

  notDownloaded: "not downloaded",
  clickToRename: "Click to rename",
  auto: "Auto",

  noResultsFor: (q) => `No results for “${q}”.`,
  speaker: (n) => `Speaker ${n}`,
  cores: (n) => `${n} ${n === 1 ? "core" : "cores"}`,
  ramGb: (gb) => `${gb} GB RAM`,
  mb: (n) => `${n} MB`,

  pickerAudioVideo: "Audio & video",
  pickerText: "Text",
};

export const common = { ru, en };
