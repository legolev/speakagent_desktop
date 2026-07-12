import { useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  FolderOpen,
  Users,
  Square,
  Clock,
  FileAudio,
  Loader2,
  Check,
  AlertCircle,
  ChevronLeft,
  X,
  Trash2,
  Copy,
  AlertTriangle,
  Search,
  Pencil,
  MapPin,
} from "lucide-react";
import { pickAudioFiles, fileInfo, probeDuration, systemInfo, revealFile } from "../lib/api";
import { useJobs, type Job, type QueueItem } from "../store/jobs";
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

function fmtDuration(sec?: number | null): string {
  if (!sec || sec <= 0) return "—";
  const s = Math.round(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(ss).padStart(2, "0")}`
    : `${m}:${String(ss).padStart(2, "0")}`;
}

function fmtDate(ms: number): string {
  return new Date(ms).toLocaleString("ru-RU", {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

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

export default function TranscribePage() {
  const jobs = useJobs((s) => s.jobs);
  const queue = useJobs((s) => s.queue);
  const enqueue = useJobs((s) => s.enqueue);
  const dequeue = useJobs((s) => s.dequeue);
  const cancel = useJobs((s) => s.cancel);
  const remove = useJobs((s) => s.remove);
  const rename = useJobs((s) => s.rename);
  const renameJob = useJobs((s) => s.renameJob);

  const [diarize, setDiarize] = useState(true);
  const [speakers, setSpeakers] = useState(0); // 0 — авто
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [editingName, setEditingName] = useState(false);
  const [nameDraft, setNameDraft] = useState("");

  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });

  const running = jobs.find((j) => j.status === "running");
  const allHistory = jobs.filter((j) => j.status !== "running");
  const q = search.trim().toLowerCase();
  const history = q ? allHistory.filter((j) => j.name.toLowerCase().includes(q)) : allHistory;

  // Секундомер для идущей записи.
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!running) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [running?.id]);
  const elapsed = running ? Math.floor((now - running.startedAt) / 1000) : 0;

  // Файлы, брошенные в окно → в очередь.
  const droppedPaths = useUi((s) => s.droppedPaths);
  const clearDropped = useUi((s) => s.clearDropped);
  useEffect(() => {
    if (!droppedPaths.length) return;
    const paths = droppedPaths;
    clearDropped();
    void addFiles(paths);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [droppedPaths]);

  async function addFiles(paths: string[]) {
    setError(null);
    try {
      const items: QueueItem[] = [];
      for (const p of paths) {
        const info = await fileInfo(p);
        const duration = await probeDuration(p).catch(() => null);
        items.push({
          path: p,
          name: info.name,
          diarize,
          durationSec: duration ?? undefined,
          numSpeakers: diarize ? speakers : undefined,
        });
      }
      enqueue(items);
    } catch (e) {
      setError(String(e));
    }
  }

  async function pick() {
    setError(null);
    try {
      const paths = await pickAudioFiles();
      if (paths.length) await addFiles(paths);
    } catch (e) {
      setError(String(e));
    }
  }

  // ─── Отдельный экран расшифровки (плеер + текст), с кнопкой «Назад» ───
  const selected = selectedId ? jobs.find((j) => j.id === selectedId) : undefined;
  if (selectedId && selected) {
    const text = selected.text ?? selected.partial;
    return (
      <div className="flex h-full flex-col">
        <div className="flex shrink-0 items-center gap-3 border-b border-white/5 px-6 py-3">
          <button
            onClick={() => {
              setSelectedId(null);
              setEditingName(false);
            }}
            className="inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-sm text-zinc-200 transition hover:bg-white/10"
          >
            <ChevronLeft size={16} /> Назад
          </button>
          <div className="min-w-0 flex-1">
            {editingName ? (
              <input
                autoFocus
                value={nameDraft}
                onChange={(e) => setNameDraft(e.target.value)}
                onBlur={() => {
                  renameJob(selected.id, nameDraft);
                  setEditingName(false);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    renameJob(selected.id, nameDraft);
                    setEditingName(false);
                  }
                  if (e.key === "Escape") setEditingName(false);
                }}
                className="w-full rounded-md border border-amber-500/40 bg-white/5 px-2 py-1 text-sm font-medium text-zinc-100 outline-none"
              />
            ) : (
              <button
                onClick={() => {
                  setNameDraft(selected.name);
                  setEditingName(true);
                }}
                title="Нажмите, чтобы переименовать"
                className="group flex max-w-full items-center gap-1.5 text-left"
              >
                <span className="truncate text-sm font-medium text-zinc-100">
                  {selected.name}
                </span>
                <Pencil
                  size={12}
                  className="shrink-0 text-zinc-500 opacity-0 transition group-hover:opacity-100"
                />
              </button>
            )}
            <button
              onClick={() => revealFile(selected.path).catch(() => {})}
              title={`Показать в системе: ${selected.path}`}
              className="mt-0.5 flex max-w-full items-center gap-1 text-left text-xs text-zinc-500 transition hover:text-amber-400"
            >
              <MapPin size={11} className="shrink-0" />
              <span className="truncate">{selected.path}</span>
            </button>
          </div>
          {selected.status === "done" && text && (
            <ExportMenu name={selected.name} text={text} names={selected.names} />
          )}
          <button
            onClick={() => {
              remove(selected.id);
              setSelectedId(null);
            }}
            className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-2.5 py-1.5 text-xs text-zinc-400 transition hover:bg-white/5 hover:text-red-400"
          >
            <Trash2 size={13} /> Удалить
          </button>
        </div>
        <div className="min-h-0 flex-1">
          {selected.status === "error" ? (
            <div className="p-6 text-sm text-red-400">{selected.error}</div>
          ) : (
            <ResultView
              path={selected.path}
              text={text}
              diarize={selected.diarize}
              names={selected.names}
              onRename={(spk, n) => rename(selected.id, spk, n)}
              withPlayer={selected.status === "done"}
              jobId={selected.status === "done" ? selected.id : undefined}
              name={selected.name}
            />
          )}
        </div>
      </div>
    );
  }

  // ─── Хаб: управление + очередь + таблица истории ───
  return (
    <div className="mx-auto w-full max-w-6xl p-6 lg:p-8">
      <h1 className="text-2xl font-semibold tracking-tight">Расшифровка</h1>
      <p className="mt-1 text-sm text-zinc-400">
        Выберите или перетащите одну или несколько записей — они встанут в очередь. Ниже —
        все ваши расшифровки, они хранятся только на этом компьютере.
      </p>

      {/* Управление */}
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <button
          onClick={pick}
          className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
        >
          <FolderOpen size={16} /> Выбрать записи
        </button>
        <label className="inline-flex cursor-pointer items-center gap-2 text-sm text-zinc-300">
          <input
            type="checkbox"
            checked={diarize}
            onChange={(e) => setDiarize(e.target.checked)}
            className="h-4 w-4 accent-amber-500"
          />
          <Users size={15} className="text-zinc-500" /> Определять, кто говорит
        </label>
        {diarize && (
          <label
            className="inline-flex items-center gap-2 text-sm text-zinc-400"
            title="Если знаете точное число говорящих — так надёжнее, чем авто"
          >
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
      {error && <div className="mt-2 text-sm text-red-400">{error}</div>}

      {/* Очередь: идёт сейчас + ожидающие */}
      {(running || queue.length > 0) && (
        <div className="glass mt-6 rounded-xl border border-white/5 p-4">
          <div className="text-sm font-medium text-zinc-300">
            В обработке
            {queue.length > 0 && (
              <span className="ml-2 text-xs text-zinc-500">+{queue.length} в очереди</span>
            )}
          </div>

          {running && (
            <div className="mt-3">
              <div className="flex items-center gap-2">
                <Loader2 size={15} className="shrink-0 animate-spin text-amber-500" />
                <span className="min-w-0 flex-1 truncate text-sm text-zinc-200">
                  {running.name}
                </span>
                <button
                  onClick={() => cancel(running.id)}
                  className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-2.5 py-1 text-xs text-zinc-300 transition hover:bg-white/5 hover:text-red-400"
                >
                  <Square size={11} /> Остановить
                </button>
              </div>
              <div className="mt-2">
                <ProgressBar stage={running.stage} done={running.done} total={running.total} />
                <div className="mt-1 text-xs text-zinc-500">
                  <Clock size={11} className="mr-1 inline" /> идёт {fmtElapsed(elapsed)}
                  {running.stage === "diarizing" && " · анализирую голоса…"}
                </div>
              </div>
            </div>
          )}

          {queue.map((q, i) => (
            <div key={`${q.path}-${i}`} className="mt-2 flex items-center gap-2">
              <Clock size={14} className="shrink-0 text-zinc-600" />
              <span className="min-w-0 flex-1 truncate text-sm text-zinc-400">{q.name}</span>
              <span className="text-xs text-zinc-600">ожидает</span>
              <button
                onClick={() => dequeue(i)}
                className="rounded-md p-1 text-zinc-500 transition hover:bg-white/5 hover:text-red-400"
                title="Убрать из очереди"
              >
                <X size={14} />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Слабое железо — предупреждение (по характеристикам машины, не по сиюминутной памяти) */}
      {sys && (sys.speed === "slow" || sys.ramTotalGb < 8) && (
        <div className="mt-3 inline-flex items-center gap-1.5 text-xs text-amber-400/80">
          <AlertTriangle size={13} />
          {sys.speed === "slow"
            ? "слабый процессор — обработка займёт больше времени"
            : "немного оперативной памяти — крупные модели могут тормозить"}
        </div>
      )}

      {/* История таблицей */}
      <div className="mt-8 flex items-center justify-between gap-3">
        <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">
          Мои расшифровки
        </h2>
        {allHistory.length > 0 && (
          <div className="relative">
            <Search
              size={14}
              className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-500"
            />
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Поиск по названию…"
              className="w-56 rounded-lg border border-white/10 bg-white/5 py-1.5 pl-8 pr-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-500/50"
            />
          </div>
        )}
      </div>

      {allHistory.length === 0 ? (
        <div className="glass mt-3 flex flex-col items-center gap-2 rounded-xl border border-white/5 p-10 text-center">
          <FileAudio size={28} className="text-zinc-700" />
          <div className="text-sm text-zinc-400">
            Здесь появятся ваши расшифровки — выберите запись выше.
          </div>
        </div>
      ) : history.length === 0 ? (
        <div className="mt-3 rounded-xl border border-white/5 p-6 text-center text-sm text-zinc-500">
          Ничего не найдено по запросу «{search}».
        </div>
      ) : (
        <div className="mt-3 overflow-hidden rounded-xl border border-white/5">
          <div className="grid grid-cols-[1fr_6rem_8rem_8rem_2.5rem] items-center gap-3 border-b border-white/5 bg-white/[0.02] px-4 py-2 text-[11px] uppercase tracking-wide text-zinc-500">
            <span>Название</span>
            <span>Длит.</span>
            <span>Статус</span>
            <span>Дата</span>
            <span />
          </div>
          {history.map((j) => {
            const text = j.text ?? j.partial;
            return (
              <div
                key={j.id}
                className="grid grid-cols-[1fr_6rem_8rem_8rem_2.5rem] items-center gap-3 border-b border-white/5 px-4 py-3 transition last:border-b-0 hover:bg-white/5"
              >
                <button
                  onClick={() => {
                    setEditingName(false);
                    setSelectedId(j.id);
                  }}
                  className="flex min-w-0 items-center gap-2 text-left"
                >
                  <FileAudio size={15} className="shrink-0 text-amber-500/80" />
                  <span className="truncate text-sm text-zinc-200">{j.name}</span>
                  {j.diarize && <Users size={12} className="shrink-0 text-zinc-600" />}
                </button>
                <span className="text-xs text-zinc-400">{fmtDuration(j.durationSec)}</span>
                <StatusBadge status={j.status} />
                <span className="text-xs text-zinc-500">{fmtDate(j.startedAt)}</span>
                <div className="flex items-center justify-end gap-1">
                  <button
                    onClick={() => navigator.clipboard.writeText(text || "")}
                    className="rounded-md p-1.5 text-zinc-500 transition hover:bg-white/10 hover:text-zinc-200"
                    title="Копировать текст"
                  >
                    <Copy size={13} />
                  </button>
                  <button
                    onClick={() => remove(j.id)}
                    className="rounded-md p-1.5 text-zinc-500 transition hover:bg-white/10 hover:text-red-400"
                    title="Удалить"
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
