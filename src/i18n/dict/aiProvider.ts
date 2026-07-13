const ru = {
  // Переключатель локально / облако.
  localLabel: "На этом компьютере",
  cloudLabel: "Облачный ИИ",

  // Строка ускорения локальных «Итогов».
  gpuAccel: (gpu: string | null) => `Ускорение видеокартой — ${gpu ?? "видеокарта"}`,
  noGpuAccel: "Без ускорения — считается на процессоре",
  gpuUnfit: (gpu: string) => ` (${gpu} не подходит)`,

  // Облачный провайдер.
  cloudIntro:
    "Саммари, протоколы и задачи будут составляться через ваш облачный ИИ (совместимый с OpenAI: OpenRouter, OpenAI и др.). Токен хранится только на этом компьютере.",
  apiUrl: "Адрес API",
  model: "Модель",
  token: "Токен (ключ доступа)",
  test: "Проверить связь",
  testing: "Проверяю…",
  testOkFallback: "ок",
  testSuccess: (reply: string) => `Связь есть — облако ответило: «${reply}»`,
};

type T = typeof ru;

const en: T = {
  localLabel: "This computer",
  cloudLabel: "Cloud AI",

  gpuAccel: (gpu) => `GPU acceleration — ${gpu ?? "GPU"}`,
  noGpuAccel: "No acceleration — running on the CPU",
  gpuUnfit: (gpu) => ` (${gpu} not supported)`,

  cloudIntro:
    "Summaries, protocols, and tasks will be produced through your cloud AI (OpenAI-compatible: OpenRouter, OpenAI, and others). The token is stored only on this computer.",
  apiUrl: "API URL",
  model: "Model",
  token: "Token (access key)",
  test: "Check connection",
  testing: "Checking…",
  testOkFallback: "ok",
  testSuccess: (reply) => `Connected — the cloud replied: “${reply}”`,
};

export const aiProvider = { ru, en };
