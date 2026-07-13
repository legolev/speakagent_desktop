// Диагностика и поддержка: ID устройства + служебная информация + issue на GitHub.
const ru = {
  deviceId: "ID устройства",
  serviceInfo: "Служебная информация",
  forQuestionOrBug: "— для вопроса или бага",
  copyAll: "Скопировать всё",
  reportOnGithub: "Сообщить о проблеме на GitHub",
  // Префилл тела issue на GitHub (diag — служебная информация, подставляется как есть).
  issueBody: (diag: string) => `Опишите проблему или вопрос здесь.\n\n---\n${diag}`,
};
type T = typeof ru;
const en: T = {
  deviceId: "Device ID",
  serviceInfo: "Diagnostic info",
  forQuestionOrBug: "— for a question or bug",
  copyAll: "Copy all",
  reportOnGithub: "Report an issue on GitHub",
  issueBody: (diag) => `Describe your problem or question here.\n\n---\n${diag}`,
};
export const diagnostics = { ru, en };
