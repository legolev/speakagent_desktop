// Витрина выбора ASR-модели: метки пригодности под железо + заметки-подсказки.
const ru = {
  // Метки пригодности (fit) модели под этот ПК.
  fitGood: "Хорошо подойдёт",
  fitOk: "Подойдёт",
  fitHeavy: "Тяжело для этого ПК",

  // Короткие заметки-подсказки к оценке пригодности.
  noteLowRam: "мало оперативной памяти",
  noteMostAccurateSlower: "самая точная, но помедленнее",
  noteVerySlowCpu: "очень медленно на этом процессоре",
  noteWillBeSlow: "будет медленно",
  noteSlightlySlower: "чуть медленнее",

  // Тултип с оценкой по конкретному железу (hw уже собран из атомов).
  hwEstimateTitle: (hw: string) => `Оценка по вашему железу: ${hw}`,
};

type T = typeof ru;

const en: T = {
  fitGood: "Good fit",
  fitOk: "Should work",
  fitHeavy: "Heavy for this PC",

  noteLowRam: "not enough RAM",
  noteMostAccurateSlower: "most accurate, but slower",
  noteVerySlowCpu: "very slow on this CPU",
  noteWillBeSlow: "will be slow",
  noteSlightlySlower: "slightly slower",

  hwEstimateTitle: (hw) => `Hardware estimate: ${hw}`,
};

export const modelSelector = { ru, en };
