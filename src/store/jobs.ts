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

interface JobsState {
  jobs: Job[];
  /** Итоги по записям: jobId → kind → состояние. Живёт в сторе — переживает навигацию. */
  results: Record<string, Partial<Record<ResultKind, ResultState>>>;
  hydrate: () => Promise<void>;
  start: (
    path: string,
    name: string,
    diarize: boolean,
    durationSec?: number,
    numSpeakers?: number,
  ) => string;
  cancel: (id: string) => void;
  rename: (id: string, speaker: number, name: string) => void;
  remove: (id: string) => void;
  clear: () => void;
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
  };
}

export const useJobs = create<JobsState>((set, get) => ({
  jobs: [],
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
            startedAt: s.createdAt,
            stageStartedAt: s.createdAt,
          };
        }),
      });
    } catch {
      /* история недоступна — не критично */
    }
  },

  start: (path, name, diarize, durationSec, numSpeakers) => {
    const id = `job_${Date.now()}_${counter++}`;
    const createdAt = Date.now();
    const job: Job = {
      id,
      name,
      path,
      diarize,
      status: "running",
      stage: "decoding",
      done: 0,
      total: 0,
      partial: "",
      names: {},
      durationSec,
      startedAt: createdAt,
      stageStartedAt: createdAt,
    };
    set((s) => ({ jobs: [job, ...s.jobs] }));

    const patch = (p: Partial<Job>) =>
      set((s) => ({ jobs: s.jobs.map((j) => (j.id === id ? { ...j, ...p } : j)) }));

    const channel = new Channel<Progress>();
    channel.onmessage = (p) => {
      const cur = get().jobs.find((j) => j.id === id);
      patch({
        stage: p.stage,
        done: p.done,
        total: p.total,
        partial: p.partial,
        ...(cur && cur.stage !== p.stage ? { stageStartedAt: Date.now() } : {}),
      });
    };

    transcribe(path, diarize, id, channel, numSpeakers)
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
      });

    return id;
  },

  cancel: (id) => {
    cancelTranscribe(id).catch(() => {});
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
