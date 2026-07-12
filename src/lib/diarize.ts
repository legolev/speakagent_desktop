// Общий разбор диаризованного текста "SpeakerN [H:MM:SS]: текст".
// Используется рендером, экспортом и переименованием спикеров.

export interface Replica {
  speaker: number;
  time: string;
  text: string;
}

const RE = /^Speaker(\d+)\s*\[([\d:]+)\]:\s*(.*)$/;

export function looksDiarized(text: string): boolean {
  return RE.test(text.split("\n").find((l) => l.trim().length > 0) ?? "");
}

export function parseReplicas(text: string): Replica[] {
  return text
    .split("\n")
    .map((l) => l.match(RE))
    .filter((m): m is RegExpMatchArray => !!m)
    .map((m) => ({ speaker: Number(m[1]), time: m[2], text: m[3] }));
}

/** Отображаемое имя спикера: заданное пользователем или «Спикер N». */
export function speakerLabel(n: number, names?: Record<number, string>): string {
  return names?.[n]?.trim() || `Спикер ${n}`;
}

/** "H:MM:SS" | "M:SS" → секунды. */
export function timeToSec(t: string): number {
  return t.split(":").map(Number).reduce((acc, v) => acc * 60 + v, 0);
}

/** Уникальные номера спикеров в тексте (по порядку появления). */
export function speakerNumbers(text: string): number[] {
  const seen: number[] = [];
  for (const r of parseReplicas(text)) {
    if (!seen.includes(r.speaker)) seen.push(r.speaker);
  }
  return seen.sort((a, b) => a - b);
}
