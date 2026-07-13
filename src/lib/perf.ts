import type { SystemInfo } from "./api";
import { tr } from "../i18n";

/** Человеческий формат длительности. Никогда не обещает «меньше минуты» —
 *  короткие оценки честно округляются вверх. */
export function fmtEta(sec: number): string {
  if (sec < 50) return tr().perf.about1min;
  const m = Math.ceil(sec / 60);
  if (m < 60) return tr().perf.aboutMin(m);
  const h = Math.floor(m / 60);
  const mm = m % 60;
  return mm ? tr().perf.aboutHM(h, mm) : tr().perf.aboutH(h);
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
