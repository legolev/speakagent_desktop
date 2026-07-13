// Экран «Расшифровка» (хаб: управление + очередь + таблица истории) и отдельный
// экран одной записи. Общие атомы (Отмена/Удалить/Назад/Остановить/Авто/поиск)
// берём из common — здесь только уникальные для страницы строки.
const ru = {
  // Статус-пилюли в таблице.
  statusRunning: "в работе",
  statusError: "ошибка",
  statusDone: "готово",

  // Модалка удаления.
  deleteTitle: "Удалить запись?",
  deleteBody:
    "Расшифровка и составленные по ней итоги будут удалены безвозвратно. Сам файл на диске не трогаем.",

  // Модалка повторной расшифровки.
  retranscribeTitle: "Расшифровать заново?",
  retranscribeBody:
    "Текущий результат для этой записи будет перезаписан, а обработка может занять время (зависит от длины записи и модели).",
  retranscribeConfirm: "Да, заново",

  // Отдельный экран записи.
  revealInSystem: (path: string) => `Показать в системе: ${path}`,

  // Заголовок + интро хаба.
  title: "Расшифровка",
  intro:
    "Выберите или перетащите одну или несколько записей — они встанут в очередь. Ниже — все ваши расшифровки, они хранятся только на этом компьютере.",

  // Управление.
  pickFiles: "Выбрать записи",
  diarizeLabel: "Определять, кто говорит",
  speakersHint: "Если знаете точное число говорящих — так надёжнее, чем авто",
  speakersLabel: "Говорящих:",

  // Очередь.
  inProgress: "В обработке",
  plusInQueue: (n: number) => `+${n} в очереди`,
  running: (elapsed: string) => `идёт ${elapsed}`,
  analyzingVoices: " · анализирую голоса…",
  waiting: "ожидает",
  removeFromQueue: "Убрать из очереди",

  // Слабое железо.
  weakCpu: "слабый процессор — обработка займёт больше времени",
  lowRam: "немного оперативной памяти — крупные модели могут тормозить",

  // История.
  historyTitle: "Мои расшифровки",
  searchPlaceholder: "Поиск по названию и тексту…",
  emptyState: "Здесь появятся ваши расшифровки — выберите запись выше.",

  // Заголовки таблицы.
  colName: "Название",
  colDuration: "Длит.",
  colStatus: "Статус",
  colDate: "Дата",

  // Тултипы.
  copyText: "Копировать текст",
};

type T = typeof ru;
const en: T = {
  statusRunning: "running",
  statusError: "error",
  statusDone: "done",

  deleteTitle: "Delete recording?",
  deleteBody:
    "The transcript and any summaries built from it will be deleted for good. The file on disk stays untouched.",

  retranscribeTitle: "Transcribe again?",
  retranscribeBody:
    "The current result for this recording will be overwritten, and processing may take a while (depending on the length and model).",
  retranscribeConfirm: "Yes, redo it",

  revealInSystem: (path) => `Reveal in file manager: ${path}`,

  title: "Transcription",
  intro:
    "Pick or drag one or more recordings — they'll join the queue. Below are all your transcripts; they're kept only on this computer.",

  pickFiles: "Choose recordings",
  diarizeLabel: "Detect who's speaking",
  speakersHint: "If you know the exact number of speakers, that's more reliable than auto",
  speakersLabel: "Speakers:",

  inProgress: "Processing",
  plusInQueue: (n) => `+${n} in queue`,
  running: (elapsed) => `running ${elapsed}`,
  analyzingVoices: " · analyzing voices…",
  waiting: "waiting",
  removeFromQueue: "Remove from queue",

  weakCpu: "weak CPU — processing will take longer",
  lowRam: "limited memory — large models may run slowly",

  historyTitle: "My transcripts",
  searchPlaceholder: "Search by name and text…",
  emptyState: "Your transcripts will appear here — choose a recording above.",

  colName: "Name",
  colDuration: "Dur.",
  colStatus: "Status",
  colDate: "Date",

  copyText: "Copy text",
};

export const transcribe = { ru, en };
