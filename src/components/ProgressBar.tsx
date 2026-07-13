import { useT } from "../i18n";

interface Props {
  stage: "decoding" | "diarizing" | "transcribing" | "punctuating" | "done";
  done: number;
  total: number;
}

export default function ProgressBar({ stage, done, total }: Props) {
  const t = useT();
  const STAGE_LABEL: Record<Props["stage"], string> = {
    decoding: t.progress.decoding,
    diarizing: t.progress.diarizing,
    transcribing: t.progress.transcribing,
    punctuating: t.progress.punctuating,
    done: t.progress.done,
  };
  const determinate = stage === "transcribing" && total > 0;
  const pct = determinate ? Math.round((done / total) * 100) : stage === "done" ? 100 : 0;

  return (
    <div>
      <div className="mb-1.5 flex items-center justify-between text-xs text-zinc-400">
        <span>{STAGE_LABEL[stage]}</span>
        {determinate && <span>{pct}%</span>}
      </div>
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/10">
        {determinate || stage === "done" ? (
          <div
            className="h-full rounded-full bg-amber-500 transition-[width] duration-300"
            style={{ width: `${pct}%` }}
          />
        ) : (
          <div className="h-full w-1/3 animate-pulse rounded-full bg-amber-500/70" />
        )}
      </div>
    </div>
  );
}
