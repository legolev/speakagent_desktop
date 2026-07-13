const ru = {
  title: "О приложении",
  tagline: "Расшифровка и диаризация речи — офлайн, на вашем компьютере",
  version: (v: string) => `Версия ${v}`,
  easterEgg:
    "🥚 Все ваши записи так и не покинули этот компьютер. Ни байта. Спасибо, что выбираете приватность!",

  // Обновления.
  updates: "Обновления",
  checking: "Проверяю наличие обновлений…",
  updateAvailable: (v: string) => `Доступна новая версия ${v}`,
  upToDate: "У вас последняя версия",
  autoCheck: "Автоматическая проверка обновлений",
  downloading: (pct: number) => `Загрузка ${pct}%`,
  updateBtn: "Обновить",
  checkBtn: "Проверить",

  // Полезное.
  sourceCode: "Исходный код",
  license: "Лицензия",
  hardware: "Железо",
  accel: "Ускорение",
  gpuAccel: (gpu: string | null) => (gpu ? `видеокарта — ${gpu}` : "видеокарта"),
  cpu: "процессор",

  // Поддержать проект.
  supportTitle: "Поддержать проект",
  supportText:
    "Приложение бесплатное и офлайн. Если оно вам помогает — можно поддержать разработку. Это не покупает поддержку или приоритет, просто помогает проекту жить.",
  copyAddrTitle: (label: string, addr: string) => `Скопировать адрес ${label}: ${addr}`,

  // Состояние компонентов.
  componentsTitle: "Состояние компонентов",
  asrLabel: "Распознавание речи",
  asrOk: "готово к работе",
  asrBad: "нужно скачать модель",
  aiLabel: "ИИ-функции (итоги встреч)",
  aiOk: "готово",
  aiBad: "не настроено",
  accelLabel: "Ускорение вычислений",
  gpuFallback: "видеокарта",
  cpuOnly: "только процессор",

  // Диагностика.
  diagnosticsTitle: "Диагностика и поддержка",
  diagnosticsIntro:
    "Уникальный ID этого компьютера и служебная информация — пригодятся, если нужно задать вопрос или сообщить о проблеме.",
  madeWith: "Сделано с ❤️ для тех, кому важна приватность записей.",
};

type T = typeof ru;

const en: T = {
  title: "About",
  tagline: "Speech transcription and diarization — offline, on your computer",
  version: (v) => `Version ${v}`,
  easterEgg:
    "🥚 None of your recordings ever left this computer. Not a single byte. Thank you for choosing privacy!",

  updates: "Updates",
  checking: "Checking for updates…",
  updateAvailable: (v) => `Version ${v} is available`,
  upToDate: "You're on the latest version",
  autoCheck: "Automatic update checks",
  downloading: (pct) => `Downloading ${pct}%`,
  updateBtn: "Update",
  checkBtn: "Check",

  sourceCode: "Source code",
  license: "License",
  hardware: "Hardware",
  accel: "Acceleration",
  gpuAccel: (gpu) => (gpu ? `GPU — ${gpu}` : "GPU"),
  cpu: "CPU",

  supportTitle: "Support the project",
  supportText:
    "The app is free and offline. If it helps you, consider supporting its development. This doesn't buy you support or priority — it just helps the project stay alive.",
  copyAddrTitle: (label, addr) => `Copy ${label} address: ${addr}`,

  componentsTitle: "Component status",
  asrLabel: "Speech recognition",
  asrOk: "ready to use",
  asrBad: "model needs downloading",
  aiLabel: "AI features (meeting summaries)",
  aiOk: "ready",
  aiBad: "not configured",
  accelLabel: "Compute acceleration",
  gpuFallback: "GPU",
  cpuOnly: "CPU only",

  diagnosticsTitle: "Diagnostics and support",
  diagnosticsIntro:
    "This computer's unique ID and technical details — handy if you need to ask a question or report a problem.",
  madeWith: "Made with ❤️ for people who care about the privacy of their recordings.",
};

export const about = { ru, en };
