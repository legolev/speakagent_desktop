const ru = {
  searchPlaceholder: "Поиск по тексту расшифровки…",
  matches: (n: number) => `совпадений: ${n}`,
  notFound: "не найдено",
  noSpeech: "(речь не распознана)",
  exportFallbackName: "Итоги",
  protoBusiness: "Деловая встреча",
  protoInterview: "Собеседование",
  clickReplica: "Кликните по реплике справа — плеер перемотается к ней.",
  retranscribeTitle: "Распознать эту запись заново (например, другой моделью)",
  transcribing: "Идёт расшифровка…",
  retranscribe: "Расшифровать заново",
};
type T = typeof ru;
const en: T = {
  searchPlaceholder: "Search the transcript…",
  matches: (n) => `${n} matches`,
  notFound: "not found",
  noSpeech: "(no speech detected)",
  exportFallbackName: "Summary",
  protoBusiness: "Business meeting",
  protoInterview: "Interview",
  clickReplica: "Click a line on the right — the player will jump to it.",
  retranscribeTitle: "Transcribe this recording again (e.g. with a different model)",
  transcribing: "Transcribing…",
  retranscribe: "Transcribe again",
};
export const resultView = { ru, en };
