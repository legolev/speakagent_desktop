import { useState } from "react";
import { Link } from "react-router-dom";
import {
  Clock,
  Check,
  Loader2,
  AlertCircle,
  Copy,
  Trash2,
  FileAudio,
  Users,
  ChevronRight,
} from "lucide-react";
import { useJobs, type Job } from "../store/jobs";
import ResultView from "../components/ResultView";
import ExportMenu from "../components/ExportMenu";

function StatusBadge({ status }: { status: Job["status"] }) {
  if (status === "running")
    return (
      <span className="inline-flex items-center gap-1 text-xs text-amber-400">
        <Loader2 size={12} className="animate-spin" /> в работе
      </span>
    );
  if (status === "error")
    return (
      <span className="inline-flex items-center gap-1 text-xs text-red-400">
        <AlertCircle size={12} /> ошибка
      </span>
    );
  return (
    <span className="inline-flex items-center gap-1 text-xs text-emerald-400">
      <Check size={12} /> готово
    </span>
  );
}

export default function HistoryPage() {
  const jobs = useJobs((s) => s.jobs);
  const remove = useJobs((s) => s.remove);
  const rename = useJobs((s) => s.rename);
  const clear = useJobs((s) => s.clear);
  const [open, setOpen] = useState<string | null>(null);

  if (jobs.length === 0) {
    return (
      <div className="mx-auto max-w-3xl p-8">
        <h1 className="text-2xl font-semibold tracking-tight">История</h1>
        <div className="glass mt-6 flex flex-col items-center gap-3 rounded-xl border border-white/5 p-12 text-center">
          <Clock size={28} className="text-zinc-600" />
          <div className="text-zinc-400">Здесь появятся ваши расшифровки.</div>
          <Link
            to="/transcribe"
            className="mt-1 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
          >
            Расшифровать запись
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-3xl p-8">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">История</h1>
        <button
          onClick={clear}
          className="text-xs text-zinc-500 transition hover:text-red-400"
        >
          Очистить историю
        </button>
      </div>
      <p className="mt-2 text-sm text-zinc-400">
        Все расшифровки хранятся только на этом компьютере.
      </p>

      <div className="mt-6 flex flex-col gap-2">
        {jobs.map((j) => {
          const expanded = open === j.id;
          const text = j.text ?? j.partial;
          return (
            <div key={j.id} className="glass rounded-xl border border-white/5">
              <button
                onClick={() => setOpen(expanded ? null : j.id)}
                className={`flex w-full items-center gap-3 p-4 text-left transition hover:bg-white/5 ${
                  expanded ? "rounded-t-xl" : "rounded-xl"
                }`}
              >
                <FileAudio size={18} className="shrink-0 text-amber-500" />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm text-zinc-200">{j.name}</div>
                  <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1">
                    <StatusBadge status={j.status} />
                    {j.diarize && (
                      <span className="inline-flex items-center gap-1 text-xs text-zinc-500">
                        <Users size={12} /> со спикерами
                      </span>
                    )}
                    <span className="text-xs text-zinc-600">
                      {new Date(j.startedAt).toLocaleString("ru-RU", {
                        day: "2-digit",
                        month: "short",
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </span>
                  </div>
                </div>
                <ChevronRight
                  size={16}
                  className={`shrink-0 text-zinc-600 transition-transform ${
                    expanded ? "rotate-90" : ""
                  }`}
                />
              </button>

              {expanded && (
                <div className="border-t border-white/5">
                  {j.status === "error" ? (
                    <div className="p-4 text-sm text-red-400">{j.error}</div>
                  ) : (
                    <>
                      <div className="h-[26rem]">
                        <ResultView
                          path={j.path}
                          text={text}
                          diarize={j.diarize}
                          names={j.names}
                          onRename={(spk, n) => rename(j.id, spk, n)}
                          withPlayer={j.status === "done"}
                          jobId={j.status === "done" ? j.id : undefined}
                          name={j.name}
                        />
                      </div>
                      <div className="flex items-center gap-2 border-t border-white/5 p-3">
                        {j.status === "done" && text && (
                          <ExportMenu name={j.name} text={text} names={j.names} />
                        )}
                        <button
                          onClick={() => navigator.clipboard.writeText(text || "")}
                          className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-3 py-1.5 text-xs text-zinc-300 transition hover:bg-white/5"
                        >
                          <Copy size={13} /> Копировать
                        </button>
                        <div className="flex-1" />
                        <button
                          onClick={() => remove(j.id)}
                          className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-3 py-1.5 text-xs text-zinc-400 transition hover:bg-white/5 hover:text-red-400"
                        >
                          <Trash2 size={13} /> Удалить
                        </button>
                      </div>
                    </>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
