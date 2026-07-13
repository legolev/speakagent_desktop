import { useState } from "react";
import { ArrowRightLeft } from "lucide-react";
import { parseReplicas, speakerLabel, timeToSec } from "../lib/diarize";
import { highlightText } from "../lib/highlight";
import { useT } from "../i18n";

interface Props {
  text: string;
  names?: Record<number, string>;
  onRename?: (speaker: number, name: string) => void;
  activeIndex?: number; // karaoke: индекс подсвеченной реплики
  onSeek?: (sec: number) => void; // клик по реплике → перемотка плеера
  query?: string; // подсветка совпадений при поиске по тексту
  speakers?: number[]; // список спикеров (для переназначения реплики)
  onReassign?: (index: number, speaker: number) => void; // переназначить реплику
}

const COLOR = [
  "text-amber-400",
  "text-sky-400",
  "text-emerald-400",
  "text-violet-400",
  "text-rose-400",
  "text-orange-400",
];
const BG = [
  "bg-amber-500/10",
  "bg-sky-500/10",
  "bg-emerald-500/10",
  "bg-violet-500/10",
  "bg-rose-500/10",
  "bg-orange-500/10",
];

export default function DiarizeRenderer({
  text,
  names,
  onRename,
  activeIndex,
  onSeek,
  query,
  speakers,
  onReassign,
}: Props) {
  const t = useT();
  const replicas = parseReplicas(text);
  const [editIdx, setEditIdx] = useState<number | null>(null);
  const [val, setVal] = useState("");
  const [moveIdx, setMoveIdx] = useState<number | null>(null);

  if (replicas.length === 0) {
    return (
      <div className="whitespace-pre-wrap text-sm leading-relaxed text-zinc-200">
        {highlightText(text, query)}
      </div>
    );
  }

  const commit = (speaker: number) => {
    onRename?.(speaker, val);
    setEditIdx(null);
  };

  return (
    <div className="flex flex-col gap-3">
      {replicas.map((r, i) => {
        const c = (r.speaker - 1) % COLOR.length;
        return (
          <div key={i} className="flex flex-col gap-1">
            <div className="flex items-center gap-2 text-xs">
              {onRename && editIdx === i ? (
                <input
                  autoFocus
                  value={val}
                  onChange={(e) => setVal(e.target.value)}
                  onBlur={() => commit(r.speaker)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") commit(r.speaker);
                    if (e.key === "Escape") setEditIdx(null);
                  }}
                  placeholder={t.common.speaker(r.speaker)}
                  className="w-40 rounded bg-white/10 px-1.5 py-0.5 text-xs text-zinc-100 outline-none ring-1 ring-amber-500/50"
                />
              ) : (
                <span
                  className={`font-medium ${COLOR[c]} ${
                    onRename ? "cursor-pointer hover:underline" : ""
                  }`}
                  title={onRename ? t.common.clickToRename : undefined}
                  onClick={() => {
                    if (!onRename) return;
                    setVal(names?.[r.speaker] ?? "");
                    setEditIdx(i);
                  }}
                >
                  {speakerLabel(r.speaker, names)}
                </span>
              )}
              <span className="text-zinc-600">{r.time}</span>
              {onReassign && speakers && speakers.length > 1 && (
                <div className="relative">
                  <button
                    onClick={() => setMoveIdx(moveIdx === i ? null : i)}
                    title={t.speakers.reassignTitle}
                    className="flex text-zinc-600 transition hover:text-zinc-300"
                  >
                    <ArrowRightLeft size={12} />
                  </button>
                  {moveIdx === i && (
                    <div className="absolute left-0 top-full z-20 mt-1 min-w-[9rem] rounded-lg border border-white/10 bg-zinc-900 p-1 shadow-xl">
                      <div className="px-2 py-1 text-[11px] text-zinc-500">{t.speakers.moveTo}</div>
                      {speakers
                        .filter((s) => s !== r.speaker)
                        .map((s) => (
                          <button
                            key={s}
                            onClick={() => {
                              onReassign(i, s);
                              setMoveIdx(null);
                            }}
                            className="block w-full rounded px-2 py-1 text-left text-xs text-zinc-200 transition hover:bg-white/10"
                          >
                            {speakerLabel(s, names)}
                          </button>
                        ))}
                    </div>
                  )}
                </div>
              )}
            </div>
            <div
              onClick={onSeek ? () => onSeek(timeToSec(r.time)) : undefined}
              className={`select-text rounded-lg ${BG[c]} px-3 py-2 text-sm leading-relaxed text-zinc-200 ${
                onSeek ? "cursor-pointer" : ""
              } ${i === activeIndex ? "ring-2 ring-amber-400/70" : ""}`}
            >
              {highlightText(r.text, query)}
            </div>
          </div>
        );
      })}
    </div>
  );
}
