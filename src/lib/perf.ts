import type { SystemInfo } from "./api";

/** Человеческий формат длительности. Никогда не обещает «меньше минуты» —
 *  короткие оценки честно округляются вверх. */
export function fmtEta(sec: number): string {
  if (sec < 50) return "~1 мин";
  const m = Math.ceil(sec / 60);
  if (m < 60) return `~${m} мин`;
  const h = Math.floor(m / 60);
  const mm = m % 60;
  return mm ? `~${h} ч ${mm} мин` : `~${h} ч`;
}

/** Оценка полного времени расшифровки: длительность × RTF режима + фиксированный
 *  оверхед (декод файла + загрузка моделей). */
export function estimateTranscribeSec(
  sys: SystemInfo,
  durationSec: number,
  diarize: boolean,
): number {
  const rtf = diarize ? sys.rtfDiar : sys.rtfPlain;
  return durationSec * rtf + sys.overheadSec;
}
