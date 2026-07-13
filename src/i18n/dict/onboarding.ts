const ru = {
  welcome: "Добро пожаловать в SpeakAgent",
  sub: "Расшифровка речи прямо на вашем компьютере",

  feat1: "Без интернета",
  feat2: "Различает голоса",
  feat3: "Ничего не уходит в сеть",

  pickTitle: "Выберите язык распознавания",
  pickHint:
    "Модель скачается один раз (размер указан ниже) — плюс конвертер аудио (~98 МБ). Остальное уже внутри приложения. Позже можно сменить в настройках.",

  downloadingModel: (pct: number) => `Скачиваю модель… ${pct}%`,

  later: "Позже",
  start: "Начать",
  recommended: "Рекомендуем",
};

type T = typeof ru;

const en: T = {
  welcome: "Welcome to SpeakAgent",
  sub: "Speech transcription, right on your computer",

  feat1: "Works offline",
  feat2: "Tells voices apart",
  feat3: "Nothing leaves your device",

  pickTitle: "Choose the recognition language",
  pickHint:
    "The model downloads once (size shown below) — plus an audio converter (~98 MB). Everything else is already built in. You can change it later in Settings.",

  downloadingModel: (pct) => `Downloading model… ${pct}%`,

  later: "Later",
  start: "Start",
  recommended: "Recommended",
};

export const onboarding = { ru, en };
