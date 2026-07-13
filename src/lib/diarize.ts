// Общий разбор диаризованного текста "SpeakerN [H:MM:SS]: текст".
// Используется рендером, экспортом и переименованием спикеров.

import { tr } from "../i18n";

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
  return names?.[n]?.trim() || tr().common.speaker(n);
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

/** Число реплик на каждого спикера (для ростера спикеров). */
export function speakerCounts(text: string): Record<number, number> {
  const out: Record<number, number> = {};
  for (const r of parseReplicas(text)) out[r.speaker] = (out[r.speaker] ?? 0) + 1;
  return out;
}

/**
 * Переписать номера спикеров в диаризованном тексте — основа для слияния и
 * переназначения реплик. `pick(speaker, replicaIndex)` возвращает новый номер спикера
 * для реплики (индекс совпадает с порядком в parseReplicas). Строки не добавляются и не
 * удаляются — таймкоды, порядок и индексы реплик сохраняются, правка долетает до рендера,
 * экспорта, поиска и LLM-итогов (текст — единый источник истины).
 */
export function rewriteSpeakers(
  text: string,
  pick: (speaker: number, index: number) => number,
): string {
  let idx = -1;
  return text
    .split("\n")
    .map((line) => {
      const m = line.match(RE);
      if (!m) return line;
      idx += 1;
      const cur = Number(m[1]);
      const next = pick(cur, idx);
      if (next === cur) return line;
      return `Speaker${next}${line.slice(`Speaker${m[1]}`.length)}`;
    })
    .join("\n");
}
