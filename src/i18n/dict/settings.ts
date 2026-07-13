const ru = {
  title: "Настройки",

  defaultModel: "Модель по умолчанию",
  defaultModelHint:
    "Выберите язык распознавания. Не скачанную модель можно загрузить прямо здесь — нажмите на неё.",

  setupWizard: "Мастер настройки",
  dataFolder: "Папка с данными",

  aiFeatures: "ИИ-функции",
  aiFeaturesHint:
    "Саммари, протокол и задачи по записи. Можно считать локально (полностью офлайн, помощник докачивается один раз ~32 МБ) или через ваш облачный ИИ по токену.",

  interfaceLanguage: "Язык интерфейса",
  interfaceLanguageHint: "Меняет язык приложения и язык «Итогов встречи».",

  privacy: "Приватность",
  privacyText:
    "Все записи обрабатываются только на этом компьютере и никуда не отправляются. Интернет нужен лишь для первой загрузки моделей (и для облачного ИИ, если вы его выбрали).",
};

type T = typeof ru;

const en: T = {
  title: "Settings",

  defaultModel: "Default model",
  defaultModelHint:
    "Choose the recognition language. A model that isn't downloaded yet can be fetched right here — just click it.",

  setupWizard: "Setup wizard",
  dataFolder: "Data folder",

  aiFeatures: "AI features",
  aiFeaturesHint:
    "Summaries, minutes, and tasks for a recording. Run them locally (fully offline, the helper downloads once, ~32 MB) or through your cloud AI with a token.",

  interfaceLanguage: "Interface language",
  interfaceLanguageHint: "Changes the app language and the language of meeting summaries.",

  privacy: "Privacy",
  privacyText:
    "All recordings are processed only on this computer and never sent anywhere. The internet is needed only for the first model download (and for cloud AI, if you chose it).",
};

export const settings = { ru, en };
