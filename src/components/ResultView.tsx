import { useEffect, useMemo, useRef, useState } from "react";
import { RotateCcw } from "lucide-react";
import AudioPlayer from "./AudioPlayer";
import DiarizeRenderer from "./DiarizeRenderer";
import { ArtifactPanel, TABS, KIND_TITLE, type Tab } from "./MeetingResults";
import { parseReplicas, timeToSec } from "../lib/diarize";
import { useJobs } from "../store/jobs";
import type { ResultKind } from "../lib/api";

interface Props {
  path: string;
  text: string;
  diarize: boolean;
  names?: Record<number, string>;
  onRename?: (speaker: number, name: string) => void;
  /** Показывать плеер + кнопки форматов (отдельной колонкой слева). */
  withPlayer?: boolean;
  /** id записи в истории — включает форматы «Итогов» (саммари/протокол/задачи). */
  jobId?: string;
  /** Имя записи (для файлов экспорта итогов). */
  name?: string;
}

export default function ResultView({
  path,
  text,
  diarize,
  names,
  onRename,
  withPlayer = false,
  jobId,
  name,
}: Props) {
  const [time, setTime] = useState(0);
  const seekRef = useRef<((sec: number) => void) | null>(null);
  const [tab, setTab] = useState<Tab>("text");
  const [protoStyle, setProtoStyle] = useState<"business" | "interview">("business");

  const hydrateResults = useJobs((s) => s.hydrateResults);
  const retranscribe = useJobs((s) => s.retranscribe);
  const busy = useJobs((s) => s.jobs.some((j) => j.status === "running"));
  useEffect(() => {
    if (jobId) void hydrateResults(jobId);
  }, [jobId, hydrateResults]);

  const starts = useMemo(() => parseReplicas(text).map((r) => timeToSec(r.time)), [text]);
  const activeIndex = useMemo(() => {
    let idx = -1;
    for (let i = 0; i < starts.length; i++) {
      if (starts[i] <= time + 0.05) idx = i;
      else break;
    }
    return idx;
  }, [starts, time]);

  // Прокручиваемый транскрипт (реплики со спикерами или сплошной текст).
  const transcript = (
    <div className="min-h-0 flex-1 overflow-y-auto p-4">
      {diarize ? (
        <DiarizeRenderer
          text={text}
          names={names}
          onRename={onRename}
          activeIndex={withPlayer ? activeIndex : undefined}
          onSeek={withPlayer ? (sec) => seekRef.current?.(sec) : undefined}
        />
      ) : (
        <div className="select-text whitespace-pre-wrap text-sm leading-relaxed text-zinc-200">
          {text || "(речь не распознана)"}
        </div>
      )}
    </div>
  );

  // Правая область: транскрипт («Текст») либо артефакт «Итогов» выбранного формата.
  const rightContent =
    !jobId || tab === "text" ? (
      <div className="flex h-full min-h-0 flex-col">{transcript}</div>
    ) : (
      <div className="flex h-full min-h-0 flex-col">
        {tab === "protocol" && (
          <div className="flex shrink-0 items-center gap-1 px-4 pt-3">
            {(["business", "interview"] as const).map((s) => (
              <button
                key={s}
                onClick={() => setProtoStyle(s)}
                className={`rounded-md px-2.5 py-1 text-xs transition ${
                  protoStyle === s
                    ? "bg-amber-500/15 text-amber-300"
                    : "text-zinc-500 hover:text-zinc-300"
                }`}
              >
                {s === "business" ? "Деловая встреча" : "Собеседование"}
              </button>
            ))}
          </div>
        )}
        <div className="min-h-0 flex-1">
          <ArtifactPanel
            jobId={jobId}
            kind={(tab === "protocol" ? protoStyle : tab) as ResultKind}
            exportName={`${name ?? "Итоги"} — ${
              KIND_TITLE[tab === "protocol" ? protoStyle : tab]
            }`}
            textLen={text.length}
          />
        </div>
      </div>
    );

  // Без плеера (идёт обработка) — текст на всю ширину.
  if (!withPlayer) return <div className="flex h-full min-h-0 flex-col">{transcript}</div>;

  // С плеером — две колонки: слева медиа + кнопки форматов, справа контент.
  return (
    <div className="flex h-full min-h-0 flex-col gap-3 p-3 lg:flex-row">
      <div className="flex shrink-0 flex-col gap-2 lg:w-[22rem]">
        <div className="rounded-xl border border-white/10 bg-black/20 p-3 lg:sticky lg:top-0">
          <AudioPlayer path={path} onTime={setTime} seekRef={seekRef} />
          {diarize && (
            <div className="mt-2 px-0.5 text-[11px] leading-snug text-zinc-500">
              Кликните по реплике справа — плеер перемотается к ней.
            </div>
          )}
        </div>
        {jobId && (
          <button
            onClick={() => retranscribe(jobId)}
            disabled={busy}
            title="Распознать эту запись заново (например, другой моделью)"
            className="flex items-center justify-center gap-2 rounded-xl border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm font-medium text-amber-300 transition hover:bg-amber-500/20 disabled:cursor-default disabled:opacity-40"
          >
            <RotateCcw size={15} /> {busy ? "Идёт расшифровка…" : "Расшифровать заново"}
          </button>
        )}
        {jobId && (
          <div className="flex flex-col gap-0.5 rounded-xl border border-white/10 bg-black/20 p-2">
            {TABS.map((t) => {
              const Icon = t.icon;
              const active = tab === t.id;
              return (
                <button
                  key={t.id}
                  onClick={() => setTab(t.id)}
                  className={`flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition ${
                    active
                      ? "bg-amber-500/15 text-amber-300"
                      : "text-zinc-400 hover:bg-white/5 hover:text-zinc-100"
                  }`}
                >
                  <Icon size={15} /> {t.label}
                </button>
              );
            })}
          </div>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-hidden rounded-xl border border-white/10 bg-black/20">
        {rightContent}
      </div>
    </div>
  );
}
