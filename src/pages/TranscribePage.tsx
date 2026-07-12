import { useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { FolderOpen, Sparkles, FileAudio, Users, Clock, Square, Gauge, AlertTriangle } from "lucide-react";
import { pickAudioFile, fileInfo, systemInfo, probeDuration } from "../lib/api";
import { fmtEta, estimateTranscribeSec } from "../lib/perf";
import { useJobs } from "../store/jobs";
import { useUi } from "../store/ui";
import ProgressBar from "../components/ProgressBar";
import ResultView from "../components/ResultView";
import ExportMenu from "../components/ExportMenu";

function fmtElapsed(sec: number): string {
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = sec % 60;
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
    : `${m}:${String(s).padStart(2, "0")}`;
}

export default function TranscribePage() {
  const jobs = useJobs((s) => s.jobs);
  const start = useJobs((s) => s.start);
  const rename = useJobs((s) => s.rename);
  const cancel = useJobs((s) => s.cancel);
  const current = jobs[0];

  const [picked, setPicked] = useState<{
    path: string;
    name: string;
    size: string;
    duration?: number;
  } | null>(null);
  const [diarize, setDiarize] = useState(true);
  const [speakers, setSpeakers] = useState(0); // 0 — авто; иначе точное число говорящих
  const [error, setError] = useState<string | null>(null);

  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });

  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (current?.status !== "running") return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [current?.status]);
  const elapsed = current ? Math.floor((now - current.startedAt) / 1000) : 0;

  const droppedPath = useUi((s) => s.droppedPath);
  const clearDropped = useUi((s) => s.clearDropped);
  useEffect(() => {
    if (!droppedPath) return;
    const p = droppedPath;
    clearDropped();
    (async () => {
      try {
        await loadPicked(p);
        setError(null);
      } catch (e) {
        setError(String(e));
      }
    })();
  }, [droppedPath, clearDropped]);

  async function loadPicked(p: string) {
    const info = await fileInfo(p);
    const duration = await probeDuration(p).catch(() => null);
    setPicked({ path: p, name: info.name, size: info.size_human, duration: duration ?? undefined });
  }

  async function pick() {
    setError(null);
    try {
      const p = await pickAudioFile();
      if (!p) return;
      await loadPicked(p);
    } catch (e) {
      setError(String(e));
    }
  }

  function run() {
    if (!picked) return;
    start(picked.path, picked.name, diarize, picked.duration, diarize ? speakers : undefined);
    setPicked(null);
  }

  const hasResult = current && current.status !== "error" && (current.text || current.partial);

  return (
    <div className="flex h-full flex-col">
      {/* Управление — закреплено сверху */}
      <div className="shrink-0 border-b border-white/5 px-8 py-5">
        <h1 className="text-xl font-semibold tracking-tight">Новая расшифровка</h1>

        <div className="mt-3 flex flex-wrap items-center gap-3">
          <button
            onClick={pick}
            className="inline-flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-sm font-medium text-zinc-100 transition hover:bg-white/10"
          >
            <FolderOpen size={16} /> Выбрать запись
          </button>
          {picked && (
            <button
              onClick={run}
              className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
            >
              <Sparkles size={16} /> Расшифровать
            </button>
          )}
          <label className="ml-1 inline-flex cursor-pointer items-center gap-2 text-sm text-zinc-300">
            <input
              type="checkbox"
              checked={diarize}
              onChange={(e) => setDiarize(e.target.checked)}
              className="h-4 w-4 accent-amber-500"
            />
            <Users size={15} className="text-zinc-500" /> Определять, кто говорит
          </label>
          {diarize && (
            <label className="inline-flex items-center gap-2 text-sm text-zinc-400" title="Если знаете точное число говорящих — так надёжнее, чем авто">
              Говорящих:
              <select
                value={speakers}
                onChange={(e) => setSpeakers(Number(e.target.value))}
                className="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5 text-sm text-zinc-100 outline-none transition hover:bg-white/10 focus:border-amber-500/50"
              >
                <option value={0}>Авто</option>
                {[2, 3, 4, 5, 6, 7, 8].map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>

        {picked && (
          <div className="mt-3 inline-flex items-center gap-2 rounded-lg border border-white/5 bg-white/5 px-3 py-1.5 text-sm">
            <FileAudio size={15} className="text-amber-500" />
            <span className="max-w-md truncate text-zinc-200" title={picked.path}>
              {picked.name}
            </span>
            <span className="text-xs text-zinc-500">{picked.size}</span>
          </div>
        )}

        {picked?.duration && sys && (
          <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs">
            <span className="inline-flex items-center gap-1 text-zinc-400">
              <Gauge size={13} className="text-amber-500/80" />
              Примерное время: {fmtEta(estimateTranscribeSec(sys, picked.duration, diarize))}
              {!sys.measured && " (первая оценка — уточнится)"}
            </span>
            {(sys.speed === "slow" || sys.ramAvailableGb < 2) && (
              <span className="inline-flex items-center gap-1 text-amber-400/80">
                <AlertTriangle size={13} />
                {sys.speed === "slow"
                  ? "слабый процессор — обработка займёт больше времени"
                  : "мало свободной памяти"}
              </span>
            )}
          </div>
        )}

        {error && <div className="mt-2 text-sm text-red-400">{error}</div>}
      </div>

      {/* Результат — занимает остаток, скроллится внутри */}
      <div className="min-h-0 flex-1 p-6">
        {!current ? (
          <div className="flex h-full flex-col items-center justify-center gap-3 text-center text-zinc-500">
            <FileAudio size={40} className="text-zinc-700" />
            <div>Выберите запись или перетащите файл в окно</div>
          </div>
        ) : (
          <div className="glass flex h-full flex-col rounded-xl border border-white/5">
            <div className="shrink-0 border-b border-white/5 p-4">
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0 truncate text-sm font-medium text-zinc-200">
                  {current.name}
                </div>
                {current.status === "done" && (current.text || current.partial) && (
                  <ExportMenu
                    name={current.name}
                    text={current.text ?? current.partial}
                    names={current.names}
                  />
                )}
              </div>

              {current.status === "running" && (
                <div className="mt-3">
                  <ProgressBar stage={current.stage} done={current.done} total={current.total} />
                  <div className="mt-2 flex items-center justify-between text-xs text-zinc-500">
                    <span className="inline-flex items-center gap-1.5">
                      <Clock size={12} /> идёт {fmtElapsed(elapsed)}
                      {current.stage === "diarizing" && " · анализирую голоса…"}
                    </span>
                    <button
                      onClick={() => cancel(current.id)}
                      className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-2.5 py-1 text-zinc-300 transition hover:bg-white/5 hover:text-red-400"
                    >
                      <Square size={11} /> Остановить
                    </button>
                  </div>
                </div>
              )}

              {current.status === "error" && (
                <div className="mt-2 text-sm text-red-400">
                  Не удалось расшифровать: {current.error}
                </div>
              )}
            </div>

            {hasResult && (
              <div className="min-h-0 flex-1">
                <ResultView
                  path={current.path}
                  text={current.text ?? current.partial}
                  diarize={current.diarize}
                  names={current.names}
                  onRename={
                    current.status === "done" ? (spk, n) => rename(current.id, spk, n) : undefined
                  }
                  withPlayer={current.status === "done"}
                  jobId={current.status === "done" ? current.id : undefined}
                  name={current.name}
                />
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
