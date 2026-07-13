const ru = {
  summarizing: "составляю итоги…",
  summarizingReading: (done: number, total: number) =>
    `составляю итоги: читаю ${done}/${total}…`,
  remaining: (eta: string) => `осталось ${eta}`,
  switchLang: "Сменить язык интерфейса",
};
type T = typeof ru;
const en: T = {
  summarizing: "summarizing…",
  summarizingReading: (done, total) => `summarizing: reading ${done}/${total}…`,
  remaining: (eta) => `remaining ${eta}`,
  switchLang: "Switch interface language",
};
export const statusBar = { ru, en };
