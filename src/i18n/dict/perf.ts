const ru = {
  about1min: "~1 мин",
  aboutMin: (m: number) => `~${m} мин`,
  aboutHM: (h: number, mm: number) => `~${h} ч ${mm} мин`,
  aboutH: (h: number) => `~${h} ч`,
};
type T = typeof ru;
const en: T = {
  about1min: "~1 min",
  aboutMin: (m) => `~${m} min`,
  aboutHM: (h, mm) => `~${h} h ${mm} min`,
  aboutH: (h) => `~${h} h`,
};
export const perf = { ru, en };
