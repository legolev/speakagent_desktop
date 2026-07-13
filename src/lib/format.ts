// Общие форматтеры для истории (расшифровки и диктовка).

import { tr } from "../i18n";

/** Длительность в формате M:SS или H:MM:SS. */
export function fmtDuration(sec?: number | null): string {
  if (!sec || sec <= 0) return "—";
  const s = Math.round(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(ss).padStart(2, "0")}`
    : `${m}:${String(ss).padStart(2, "0")}`;
}

/** Дата/время в короткой русской локали. */
export function fmtDate(ms: number): string {
  return new Date(ms).toLocaleString(tr().common.locale, {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}
