import { useState } from "react";
import { Users, ChevronDown } from "lucide-react";
import { speakerNumbers, speakerCounts, speakerLabel } from "../lib/diarize";
import { useT } from "../i18n";

const DOT = [
  "bg-amber-400",
  "bg-sky-400",
  "bg-emerald-400",
  "bg-violet-400",
  "bg-rose-400",
  "bg-orange-400",
];

interface Props {
  text: string;
  names?: Record<number, string>;
  /** Объединить спикера `from` в `into`. */
  onMerge: (from: number, into: number) => void;
}

/** Ростер спикеров: фишки (цвет + имя + число реплик) с действием «объединить с…». */
export default function SpeakerRoster({ text, names, onMerge }: Props) {
  const t = useT();
  const speakers = speakerNumbers(text);
  const counts = speakerCounts(text);
  const [openFor, setOpenFor] = useState<number | null>(null);

  if (speakers.length < 2) return null; // объединять нечего

  return (
    <div className="flex flex-wrap items-center gap-1.5 border-b border-white/5 px-3 py-2">
      <Users size={13} className="shrink-0 text-zinc-500" />
      {speakers.map((n) => {
        const c = (n - 1) % DOT.length;
        return (
          <div key={n} className="relative">
            <button
              onClick={() => setOpenFor(openFor === n ? null : n)}
              title={t.speakers.mergeTitle}
              className="inline-flex items-center gap-1.5 rounded-lg border border-white/10 bg-white/5 px-2 py-1 text-xs text-zinc-200 transition hover:bg-white/10"
            >
              <span className={`h-2 w-2 rounded-full ${DOT[c]}`} />
              {speakerLabel(n, names)}
              <span className="text-zinc-500">{counts[n] ?? 0}</span>
              <ChevronDown size={11} className="text-zinc-500" />
            </button>
            {openFor === n && (
              <div className="absolute left-0 top-full z-20 mt-1 min-w-[10rem] rounded-lg border border-white/10 bg-zinc-900 p-1 shadow-xl">
                <div className="px-2 py-1 text-[11px] text-zinc-500">{t.speakers.mergeWith}</div>
                {speakers
                  .filter((o) => o !== n)
                  .map((o) => (
                    <button
                      key={o}
                      onClick={() => {
                        onMerge(n, o);
                        setOpenFor(null);
                      }}
                      className="flex w-full items-center gap-1.5 rounded px-2 py-1 text-left text-xs text-zinc-200 transition hover:bg-white/10"
                    >
                      <span className={`h-2 w-2 rounded-full ${DOT[(o - 1) % DOT.length]}`} />
                      {speakerLabel(o, names)}
                    </button>
                  ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
