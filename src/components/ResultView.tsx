import { useMemo, useRef, useState } from "react";
import AudioPlayer from "./AudioPlayer";
import DiarizeRenderer from "./DiarizeRenderer";
import MeetingResults from "./MeetingResults";
import { parseReplicas, timeToSec } from "../lib/diarize";

interface Props {
  path: string;
  text: string;
  diarize: boolean;
  names?: Record<number, string>;
  onRename?: (speaker: number, name: string) => void;
  /** Показывать плеер (закреплён сверху, транскрипт скроллится под ним). */
  withPlayer?: boolean;
  /** id записи в истории — включает вкладки «Итогов» (саммари/протокол/задачи). */
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

  const starts = useMemo(() => parseReplicas(text).map((r) => timeToSec(r.time)), [text]);
  const activeIndex = useMemo(() => {
    let idx = -1;
    for (let i = 0; i < starts.length; i++) {
      if (starts[i] <= time + 0.05) idx = i;
      else break;
    }
    return idx;
  }, [starts, time]);

  const inner = (
    <div className="flex h-full min-h-0 flex-col">
      {withPlayer && (
        <div className="shrink-0 px-4 pt-4">
          <AudioPlayer path={path} onTime={setTime} seekRef={seekRef} />
        </div>
      )}

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
    </div>
  );

  // с jobId результат оборачивается во вкладки «Итогов» (текст — первая вкладка)
  if (!jobId) return inner;
  return (
    <MeetingResults jobId={jobId} name={name ?? "Итоги"} textLen={text.length}>
      {inner}
    </MeetingResults>
  );
}
