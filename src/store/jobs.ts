import { create } from "zustand";
import { Channel } from "@tauri-apps/api/core";
import {
  transcribe,
  cancelTranscribe,
  listJobs,
  saveJob,
  deleteJob,
  clearJobs,
  llmGenerate,
  cancelLlm,
  listResults,
  saveResultText,
  llmDisplayName,
  type Progress,
  type StoredJob,
  type LlmProgress,
  type ResultKind,
} from "../lib/api";

export type JobStatus = "running" | "done" | "error";

/** Состояние одного артефакта «Итогов» (саммари/протокол/задачи) для записи. */
export interface ResultState {
  status: "idle" | "running" | "done" | "error";
  stage?: LlmProgress["stage"];
  done: number;
  total: number;
  partial?: string;
  text?: string;
  error?: string;
}

export interface Job {
  id: string;
  name: string;
  path: string;
  diarize: boolean;
  status: JobStatus;
  stage: Progress["stage"];
  done: number;
  total: number;
  partial: string;
  text?: string;
  error?: string;
  names: Record<number, string>; // переименования спикеров
  durationSec?: number; // длительность записи (для оценки времени)
  startedAt: number;
  /** Когда началась текущая стадия — для честного «осталось» по скорости прогресса. */
  stageStartedAt: number;
}

/** Запись, ожидающая обработки в очереди (несколько файлов подряд). */
export interface QueueItem {
  path: string;
  name: string;
  diarize: boolean;
  durationSec?: number;
  numSpeakers?: number;
}

interface JobsState {
  jobs: Job[];
  /** Очередь на обработку — файлы обрабатываются по одному (тяжёлая CPU-работа). */
  queue: QueueItem[];
  /** Итоги по записям: jobId → kind → состояние. Живёт в сторе — переживает навигацию. */
  results: Record<string, Partial<Record<ResultKind, ResultState>>>;
  hydrate: () => Promise<void>;
  /** Добавить записи в очередь (и запустить обработку, если простаивает). */
  enqueue: (items: QueueItem[]) => void;
  /** Запустить следующий элемент очереди, если сейчас ничего не обрабатывается. */
  pump: () => void;
  /** Убрать элемент очереди по индексу (ещё не начатый). */
  dequeue: (index: number) => void;
  cancel: (id: string) => void;
  rename: (id: string, speaker: number, name: string) => void;
  /** Переименовать саму запись (её название в истории). */
  renameJob: (id: string, name: string) => void;
  remove: (id: string) => void;
  clear: () => void;
  /** Перезапустить расшифровку существующей записи (той же id) — «расшифровать заново». */
  retranscribe: (id: string) => void;
  /** Подтянуть сохранённые итоги записи из SQLite (при открытии). */
  hydrateResults: (jobId: string) => Promise<void>;
  /** Запустить генерацию итога. */
  generateResult: (jobId: string, kind: ResultKind) => void;
  cancelResult: (jobId: string) => void;
  /** Сохранить правку итога (чекбоксы задач). */
  saveResultEdit: (jobId: string, kind: ResultKind, text: string) => void;
}

let counter = 0;

function toStored(j: Job): StoredJob {
  return {
    id: j.id,
    name: j.name,
    path: j.path,
    diarize: j.diarize,
    status: (j.status === "error" ? "error" : "done") as "done" | "error",
    text: j.text ?? j.partial ?? "",
    error: j.error ?? "",
    createdAt: j.startedAt,
    speakers: JSON.stringify(j.names ?? {}),
    durationSec: j.durationSec ?? null,
  };
}

export const useJobs = create<JobsState>((set, get) => ({
  jobs: [],
  queue: [],
  results: {},

  hydrate: async () => {
    try {
      const stored = await listJobs();
      set({
        jobs: stored.map((s) => {
          let names: Record<number, string> = {};
          try {
            names = JSON.parse(s.speakers || "{}");
          } catch {
            /* ignore */
          }
          return {
            id: s.id,
            name: s.name,
            path: s.path,
            diarize: s.diarize,
            status: s.status,
            stage: "done",
            done: 1,
            total: 1,
            partial: s.text,
            text: s.status === "done" ? s.text : undefined,
            error: s.status === "error" ? s.error : undefined,
            names,
            durationSec: s.durationSec ?? undefined,
            startedAt: s.createdAt,
            stageStartedAt: s.createdAt,
          };
        }),
      });
    } catch {
      /* история недоступна — не критично */
    }
  },

  enqueue: (items) => {
    if (!items.length) return;
    set((s) => ({ queue: [...s.queue, ...items] }));
    get().pump();
  },

  dequeue: (index) => {
    set((s) => ({ queue: s.queue.filter((_, i) => i !== index) }));
  },

  pump: () => {
    const s = get();
    // одна тяжёлая расшифровка за раз
    if (s.jobs.some((j) => j.status === "running") || s.queue.length === 0) return;

    const [item, ...rest] = s.queue;
    set({ queue: rest });

    const id = `job_${Date.now()}_${counter++}`;
    const createdAt = Date.now();
    const job: Job = {
      id,
      name: item.name,
      path: item.path,
      diarize: item.diarize,
      status: "running",
      stage: "decoding",
      done: 0,
      total: 0,
      partial: "",
      names: {},
      durationSec: item.durationSec,
      startedAt: createdAt,
      stageStartedAt: createdAt,
    };
    set((st) => ({ jobs: [job, ...st.jobs] }));

    const patch = (p: Partial<Job>) =>
      set((st) => ({ jobs: st.jobs.map((j) => (j.id === id ? { ...j, ...p } : j)) }));

    const channel = new Channel<Progress>();
    channel.onmessage = (p) => {
      const cur = get().jobs.find((j) => j.id === id);
      patch({
        stage: p.stage,
        done: p.done,
        total: p.total,
        partial: p.partial,
        ...(cur && cur.stage !== p.stage ? { stageStartedAt: Date.now() } : {}),
        ...(p.audioSec > 0 ? { durationSec: p.audioSec } : {}),
      });
    };

    transcribe(item.path, item.diarize, id, channel, item.numSpeakers)
      .then((text) => {
        patch({ status: "done", text, partial: text, stage: "done" });
        const j = get().jobs.find((x) => x.id === id);
        if (j) saveJob(toStored({ ...j, status: "done", text })).catch(() => {});
      })
      .catch((e) => {
        const error = String(e);
        patch({ status: "error", error });
        const j = get().jobs.find((x) => x.id === id);
        if (j) saveJob(toStored({ ...j, status: "error", error })).catch(() => {});
      })
      .finally(() => {
        // следующий из очереди
        get().pump();
      });
  },

  cancel: (id) => {
    cancelTranscribe(id).catch(() => {});
  },

  retranscribe: (id) => {
    const s = get();
    if (s.jobs.some((j) => j.status === "running")) return; // занято — кнопка будет отключена
    const job = s.jobs.find((j) => j.id === id);
    if (!job) return;

    const patch = (p: Partial<Job>) =>
      set((st) => ({ jobs: st.jobs.map((j) => (j.id === id ? { ...j, ...p } : j)) }));

    const startedAt = Date.now();
    patch({
      status: "running",
      stage: "decoding",
      done: 0,
      total: 0,
      partial: "",
      text: undefined,
      error: undefined,
      startedAt,
      stageStartedAt: startedAt,
    });

    const channel = new Channel<Progress>();
    channel.onmessage = (p) => {
      const cur = get().jobs.find((j) => j.id === id);
      patch({
        stage: p.stage,
        done: p.done,
        total: p.total,
        partial: p.partial,
        ...(cur && cur.stage !== p.stage ? { stageStartedAt: Date.now() } : {}),
        ...(p.audioSec > 0 ? { durationSec: p.audioSec } : {}),
      });
    };

    transcribe(job.path, job.diarize, id, channel)
      .then((text) => {
        patch({ status: "done", text, partial: text, stage: "done" });
        const j = get().jobs.find((x) => x.id === id);
        if (j) saveJob(toStored({ ...j, status: "done", text })).catch(() => {});
      })
      .catch((e) => {
        const error = String(e);
        patch({ status: "error", error });
        const j = get().jobs.find((x) => x.id === id);
        if (j) saveJob(toStored({ ...j, status: "error", error })).catch(() => {});
      })
      .finally(() => get().pump());
  },

  rename: (id, speaker, name) => {
    set((s) => ({
      jobs: s.jobs.map((j) => {
        if (j.id !== id) return j;
        const names = { ...j.names };
        if (name.trim()) names[speaker] = name.trim();
        else delete names[speaker];
        return { ...j, names };
      }),
    }));
    const j = get().jobs.find((x) => x.id === id);
    if (j && j.status !== "running") saveJob(toStored(j)).catch(() => {});
  },

  renameJob: (id, name) => {
    const trimmed = name.trim();
    if (!trimmed) return;
    set((s) => ({ jobs: s.jobs.map((j) => (j.id === id ? { ...j, name: trimmed } : j)) }));
    const j = get().jobs.find((x) => x.id === id);
    if (j && j.status !== "running") saveJob(toStored(j)).catch(() => {});
  },

  remove: (id) => {
    set((s) => {
      const results = { ...s.results };
      delete results[id];
      return { jobs: s.jobs.filter((j) => j.id !== id), results };
    });
    deleteJob(id).catch(() => {});
  },

  clear: () => {
    set({ jobs: [], results: {} });
    clearJobs().catch(() => {});
  },

  // ── «Итоги встречи» ──

  hydrateResults: async (jobId) => {
    try {
      const stored = await listResults(jobId);
      if (!stored.length) return;
      set((s) => {
        const cur = { ...(s.results[jobId] ?? {}) };
        for (const r of stored) {
          const kind = r.kind as ResultKind;
          // не перетираем идущую генерацию
          if (cur[kind]?.status === "running") continue;
          cur[kind] = { status: "done", done: 0, total: 0, text: r.text };
        }
        return { results: { ...s.results, [jobId]: cur } };
      });
    } catch {
      /* итоги недоступны — не критично */
    }
  },

  generateResult: (jobId, kind) => {
    const patch = (p: Partial<ResultState>) =>
      set((s) => {
        const cur = s.results[jobId] ?? {};
        const prev: ResultState = cur[kind] ?? { status: "idle", done: 0, total: 0 };
        return {
          results: { ...s.results, [jobId]: { ...cur, [kind]: { ...prev, ...p } } },
        };
      });

    patch({ status: "running", stage: "starting", done: 0, total: 0, partial: "", error: undefined });

    const channel = new Channel<LlmProgress>();
    channel.onmessage = (p) =>
      patch({ stage: p.stage, done: p.done, total: p.total, partial: p.partial });

    llmGenerate(jobId, kind, channel)
      .then((text) => {
        patch({ status: "done", text, partial: undefined });
        // Помощник уже прогрет — почти бесплатно придумаем записи осмысленное имя
        // (только если оно ещё «файловое», т.е. пользователь его не трогал).
        const job = get().jobs.find((j) => j.id === jobId);
        if (job && /\.[a-z0-9]{2,5}$/i.test(job.name)) {
          llmDisplayName(jobId)
            .then((name) => {
              set((s) => ({
                jobs: s.jobs.map((j) => (j.id === jobId ? { ...j, name } : j)),
              }));
              const fresh = get().jobs.find((j) => j.id === jobId);
              if (fresh && fresh.status !== "running") saveJob(toStored(fresh)).catch(() => {});
            })
            .catch(() => {});
        }
      })
      .catch((e) => {
        const msg = String(e);
        if (msg.includes("отменено")) patch({ status: "idle", partial: undefined });
        else patch({ status: "error", error: msg, partial: undefined });
      });
  },

  cancelResult: (jobId) => {
    cancelLlm(jobId).catch(() => {});
  },

  saveResultEdit: (jobId, kind, text) => {
    set((s) => {
      const cur = s.results[jobId] ?? {};
      const prev: ResultState = cur[kind] ?? { status: "done", done: 0, total: 0 };
      return { results: { ...s.results, [jobId]: { ...cur, [kind]: { ...prev, text } } } };
    });
    saveResultText(jobId, kind, text).catch(() => {});
  },
}));
