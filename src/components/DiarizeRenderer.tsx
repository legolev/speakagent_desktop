import { useState } from "react";
import { parseReplicas, speakerLabel, timeToSec } from "../lib/diarize";
import { highlightText } from "../lib/highlight";

interface Props {
  text: string;
  names?: Record<number, string>;
  onRename?: (speaker: number, name: string) => void;
  activeIndex?: number; // karaoke: индекс подсвеченной реплики
  onSeek?: (sec: number) => void; // клик по реплике → перемотка плеера
  query?: string; // подсветка совпадений при поиске по тексту
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
}: Props) {
  const replicas = parseReplicas(text);
  const [editIdx, setEditIdx] = useState<number | null>(null);
  const [val, setVal] = useState("");

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
                  placeholder={`Спикер ${r.speaker}`}
                  className="w-40 rounded bg-white/10 px-1.5 py-0.5 text-xs text-zinc-100 outline-none ring-1 ring-amber-500/50"
                />
              ) : (
                <span
                  className={`font-medium ${COLOR[c]} ${
                    onRename ? "cursor-pointer hover:underline" : ""
                  }`}
                  title={onRename ? "Нажмите, чтобы переименовать" : undefined}
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
