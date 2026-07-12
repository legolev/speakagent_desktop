import type { ReactNode } from "react";

/** Обернуть совпадения `query` в тексте в <mark> (регистронезависимо). */
export function highlightText(text: string, query?: string): ReactNode {
  const q = (query ?? "").trim();
  if (!q) return text;
  const lower = text.toLowerCase();
  const ql = q.toLowerCase();
  const parts: ReactNode[] = [];
  let i = 0;
  let key = 0;
  let idx = lower.indexOf(ql, i);
  while (idx !== -1) {
    if (idx > i) parts.push(text.slice(i, idx));
    parts.push(
      <mark key={key++} className="rounded bg-amber-400/30 px-0.5 text-amber-100">
        {text.slice(idx, idx + q.length)}
      </mark>,
    );
    i = idx + q.length;
    idx = lower.indexOf(ql, i);
  }
  if (i < text.length) parts.push(text.slice(i));
  return parts;
}

/** Число совпадений `query` в тексте. */
export function countMatches(text: string, query?: string): number {
  const q = (query ?? "").trim().toLowerCase();
  if (!q) return 0;
  return text.toLowerCase().split(q).length - 1;
}
