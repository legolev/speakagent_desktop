interface Props {
  stage: "decoding" | "diarizing" | "transcribing" | "punctuating" | "done";
  done: number;
  total: number;
}

const STAGE_LABEL: Record<Props["stage"], string> = {
  decoding: "Готовлю аудио…",
  diarizing: "Определяю говорящих…",
  transcribing: "Распознаю речь…",
  punctuating: "Расставляю знаки препинания…",
  done: "Готово",
};

export default function ProgressBar({ stage, done, total }: Props) {
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
