// «Итоги встречи»: вкладки над результатом расшифровки — Текст · Саммари · Протокол · Задачи.
// Артефакты генерируются локальным LLM (бэк), стримятся сюда, живут в SQLite.
import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Channel } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  AlignLeft,
  ClipboardCopy,
  Download,
  ListChecks,
  Loader2,
  NotebookPen,
  RefreshCw,
  ScrollText,
  Sparkles,
  Square,
} from "lucide-react";
import {
  llmReady,
  ensureLlm,
  llmExportPrompt,
  systemInfo,
  type DlProgress,
  type ResultKind,
} from "../lib/api";
import { useJobs } from "../store/jobs";
import { exportArtifactMd, exportArtifactPdf, exportArtifactTxt } from "../lib/exporters";
import { useT, tr } from "../i18n";

type TDict = ReturnType<typeof useT>;

export type Tab = "text" | "summary" | "protocol" | "todo";

// Вкладки: id + иконка (код). Подписи берём из словаря — см. `tabLabel`.
export const TAB_DEFS: { id: Tab; icon: typeof AlignLeft }[] = [
  { id: "text", icon: AlignLeft },
  { id: "summary", icon: ScrollText },
  { id: "protocol", icon: NotebookPen },
  { id: "todo", icon: ListChecks },
];

/** Подпись вкладки на текущем языке. */
export function tabLabel(t: TDict, id: Tab): string {
  const m: Record<Tab, string> = {
    text: t.meeting.tabText,
    summary: t.meeting.tabSummary,
    protocol: t.meeting.tabProtocol,
    todo: t.meeting.tabTodo,
  };
  return m[id];
}

/** Заголовок артефакта (для имён экспортных файлов) на текущем языке. */
export function kindTitle(t: TDict, kind: string): string {
  const m: Record<string, string> = {
    summary: t.meeting.kindSummary,
    business: t.meeting.kindBusiness,
    interview: t.meeting.kindInterview,
    todo: t.meeting.kindTodo,
  };
  return m[kind] ?? "";
}

interface Props {
  jobId: string;
  name: string; // имя записи — для экспортных файлов
  textLen: number; // длина расшифровки — для оценки времени генерации
  children: React.ReactNode; // вкладка «Текст» (плеер + транскрипт)
}

export default function MeetingResults({ jobId, name, textLen, children }: Props) {
  const t = useT();
  const [tab, setTab] = useState<Tab>("text");
  const [protoStyle, setProtoStyle] = useState<"business" | "interview">("business");
  const hydrateResults = useJobs((s) => s.hydrateResults);

  useEffect(() => {
    void hydrateResults(jobId);
  }, [jobId, hydrateResults]);

  const kind: ResultKind | null =
    tab === "summary" ? "summary" : tab === "todo" ? "todo" : tab === "protocol" ? protoStyle : null;

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-1 border-b border-white/5 px-3 pt-2">
        {TAB_DEFS.map((d) => {
          const Icon = d.icon;
          const active = tab === d.id;
          return (
            <button
              key={d.id}
              onClick={() => setTab(d.id)}
              className={`inline-flex items-center gap-1.5 rounded-t-lg px-3 py-2 text-xs transition ${
                active
                  ? "border-b-2 border-amber-500 text-zinc-100"
                  : "text-zinc-500 hover:text-zinc-300"
              }`}
            >
              <Icon size={13} /> {tabLabel(t, d.id)}
            </button>
          );
        })}
        {tab === "protocol" && (
          <select
            value={protoStyle}
            onChange={(e) => setProtoStyle(e.target.value as "business" | "interview")}
            className="ml-auto mb-1 rounded-md border border-white/10 bg-white/5 px-2 py-1 text-xs text-zinc-300 outline-none"
          >
            <option value="business">{t.meeting.styleBusiness}</option>
            <option value="interview">{t.meeting.styleInterview}</option>
          </select>
        )}
      </div>

      <div className="min-h-0 flex-1">
        {tab === "text" ? (
          children
        ) : (
          <ArtifactPanel
            jobId={jobId}
            kind={kind!}
            exportName={`${name} — ${kindTitle(t, kind!)}`}
            textLen={textLen}
          />
        )}
      </div>
    </div>
  );
}

// ── Панель одного артефакта ──

export function ArtifactPanel({
  jobId,
  kind,
  exportName,
  textLen,
}: {
  jobId: string;
  kind: ResultKind;
  exportName: string;
  textLen: number;
}) {
  const t = useT();
  const state = useJobs((s) => s.results[jobId]?.[kind]) ?? {
    status: "idle" as const,
    done: 0,
    total: 0,
  };
  const generate = useJobs((s) => s.generateResult);
  const cancel = useJobs((s) => s.cancelResult);
  const saveEdit = useJobs((s) => s.saveResultEdit);

  const { data: ready, refetch: refetchReady } = useQuery({
    queryKey: ["llmReady"],
    queryFn: llmReady,
  });
  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });

  if (ready === false)
    return (
      <DownloadHelper onDone={() => refetchReady()}>
        <CopyPromptButton jobId={jobId} kind={kind} subtle />
      </DownloadHelper>
    );

  if (state.status === "running") {
    return (
      <div className="flex h-full flex-col">
        <div className="flex shrink-0 items-center gap-3 px-4 pt-4 text-sm text-zinc-400">
          <Loader2 size={15} className="animate-spin text-amber-500" />
          {state.stage === "reading"
            ? t.meeting.reading(state.done, state.total)
            : state.stage === "writing"
              ? t.meeting.writing
              : t.meeting.starting}
          <button
            onClick={() => cancel(jobId)}
            className="ml-auto inline-flex items-center gap-1.5 rounded-md border border-white/10 px-2.5 py-1 text-xs text-zinc-300 transition hover:bg-white/5 hover:text-red-400"
          >
            <Square size={11} /> {t.common.stop}
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          {state.partial ? (
            <Markdown text={state.partial} />
          ) : (
            <div className="text-sm text-zinc-600">
              {state.stage === "reading" ? t.meeting.readingHint : t.meeting.writingHint}
            </div>
          )}
        </div>
      </div>
    );
  }

  if (state.status === "done" && state.text) {
    return (
      <div className="flex h-full flex-col">
        <div className="flex shrink-0 items-center gap-2 px-4 pt-3">
          <ArtifactExport name={exportName} md={state.text} />
          <button
            onClick={() => navigator.clipboard.writeText(state.text ?? "")}
            className="rounded-md border border-white/10 px-3 py-1.5 text-xs text-zinc-300 transition hover:bg-white/5"
          >
            {t.common.copy}
          </button>
          <div className="flex-1" />
          <button
            onClick={() => generate(jobId, kind)}
            className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-3 py-1.5 text-xs text-zinc-400 transition hover:bg-white/5"
          >
            <RefreshCw size={12} /> {t.meeting.regenerate}
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          <Markdown
            text={state.text}
            onToggleCheckbox={(line) => {
              const next = toggleCheckboxAtLine(state.text!, line);
              saveEdit(jobId, kind, next);
            }}
          />
        </div>
      </div>
    );
  }

  // idle / error
  const eta = estimateGenMinutes(textLen, sys?.speed);
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
      {state.status === "error" && (
        <div className="max-w-md text-sm text-red-400">{state.error}</div>
      )}
      <button
        onClick={() => generate(jobId, kind)}
        className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
      >
        <Sparkles size={15} /> {state.status === "error" ? t.meeting.tryAgain : t.meeting.compose}
      </button>
      <div className="max-w-sm text-xs text-zinc-600">{t.meeting.etaLocal(eta)}</div>
      <CopyPromptButton jobId={jobId} kind={kind} subtle={sys?.speed !== "slow"} />
    </div>
  );
}

/** Грубая, но честная оценка времени генерации по длине текста и классу машины. */
function estimateGenMinutes(chars: number, speed?: "fast" | "medium" | "slow"): string {
  const ktok = chars / 2.2 / 1000; // ~2.2 символа на токен для русского
  const perKtokMin = speed === "slow" ? 1.6 : speed === "medium" ? 0.8 : 0.4;
  const lo = Math.max(1, Math.round(ktok * perKtokMin + (speed === "slow" ? 2 : 1)));
  const hi = Math.max(lo + 1, Math.round(lo * 1.8));
  return tr().meeting.minRange(lo, hi);
}

/** Запасной путь: скопировать готовый запрос (промпт + расшифровка) для внешнего ИИ. */
function CopyPromptButton({
  jobId,
  kind,
  subtle,
}: {
  jobId: string;
  kind: ResultKind;
  subtle?: boolean;
}) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  async function copy() {
    try {
      const text = await llmExportPrompt(jobId, kind);
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2500);
    } catch {
      /* ignore */
    }
  }
  return (
    <div className="flex flex-col items-center gap-1">
      <button
        onClick={copy}
        className={`inline-flex items-center gap-1.5 rounded-md border border-white/10 px-3 py-1.5 text-xs transition hover:bg-white/5 ${
          subtle ? "text-zinc-500" : "text-zinc-300"
        }`}
      >
        <ClipboardCopy size={12} /> {copied ? t.common.copiedBang : t.meeting.copyPrompt}
      </button>
      <div className="max-w-xs text-[11px] leading-snug text-zinc-600">
        {t.meeting.copyPromptHint}
      </div>
    </div>
  );
}

// ── Первичная докачка помощника (движок + модель) ──

function DownloadHelper({
  onDone,
  children,
}: {
  onDone: () => void;
  children?: React.ReactNode;
}) {
  const t = useT();
  const [pct, setPct] = useState<number | null>(null);
  const [error, setError] = useState("");

  async function run() {
    setError("");
    setPct(0);
    const ch = new Channel<DlProgress>();
    ch.onmessage = (d) => setPct(d.total > 0 ? d.done / d.total : 0);
    try {
      await ensureLlm(ch);
      onDone();
    } catch (e) {
      setError(String(e));
    } finally {
      setPct(null);
    }
  }

  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
      <div className="max-w-md text-sm text-zinc-400">{t.meeting.helperNeeded}</div>
      {error && <div className="max-w-md text-xs text-red-400">{error}</div>}
      {pct === null ? (
        <button
          onClick={run}
          className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
        >
          <Download size={15} /> {t.meeting.downloadHelper}
        </button>
      ) : (
        <div className="w-64">
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/10">
            <div
              className="h-full rounded-full bg-amber-500 transition-[width]"
              style={{ width: `${Math.round(pct * 100)}%` }}
            />
          </div>
          <div className="mt-1.5 text-xs text-zinc-500">{Math.round(pct * 100)}%</div>
        </div>
      )}
      {children}
    </div>
  );
}

// ── Markdown-рендер (стили под тёмную тему, интерактивные чекбоксы задач) ──

function Markdown({
  text,
  onToggleCheckbox,
}: {
  text: string;
  onToggleCheckbox?: (line: number) => void;
}) {
  return (
    <div className="select-text text-sm leading-relaxed text-zinc-200">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: (p) => <h1 className="mb-3 mt-1 text-lg font-semibold text-zinc-100" {...p} />,
          h2: (p) => <h2 className="mb-2 mt-4 text-base font-semibold text-zinc-100" {...p} />,
          h3: (p) => <h3 className="mb-2 mt-3 text-sm font-semibold text-zinc-200" {...p} />,
          p: (p) => <p className="mb-2" {...p} />,
          ul: (p) => <ul className="mb-3 ml-1 flex list-none flex-col gap-1.5" {...p} />,
          ol: (p) => <ol className="mb-3 ml-5 flex list-decimal flex-col gap-1.5" {...p} />,
          li: (p) => <li className="[&>p]:mb-0" {...p} />,
          strong: (p) => <strong className="font-semibold text-zinc-100" {...p} />,
          hr: () => <hr className="my-3 border-white/10" />,
          table: (p) => (
            <div className="mb-3 overflow-x-auto">
              <table className="w-full border-collapse text-xs" {...p} />
            </div>
          ),
          th: (p) => (
            <th className="border border-white/10 bg-white/5 px-2 py-1.5 text-left font-medium text-zinc-300" {...p} />
          ),
          td: (p) => <td className="border border-white/10 px-2 py-1.5 align-top" {...p} />,
          input: ({ node, checked }) => (
            <input
              type="checkbox"
              checked={!!checked}
              disabled={!onToggleCheckbox}
              onChange={() => {
                const line = node?.position?.start.line;
                if (line && onToggleCheckbox) onToggleCheckbox(line);
              }}
              className="mr-2 h-3.5 w-3.5 cursor-pointer accent-amber-500 align-middle"
            />
          ),
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}

/** Переключить чекбокс «- [ ]» ↔ «- [x]» на указанной строке markdown-источника. */
function toggleCheckboxAtLine(md: string, line: number): string {
  const lines = md.split("\n");
  const i = line - 1;
  if (i >= 0 && i < lines.length) {
    if (lines[i].includes("[ ]")) lines[i] = lines[i].replace("[ ]", "[x]");
    else lines[i] = lines[i].replace(/\[[xX]\]/, "[ ]");
  }
  return lines.join("\n");
}

// ── Экспорт артефакта ──

function ArtifactExport({ name, md }: { name: string; md: string }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const items = useMemo(
    () => [
      { label: "Markdown (.md)", fn: () => exportArtifactMd(name, md) },
      { label: t.meeting.exportTxt, fn: () => exportArtifactTxt(name, md) },
      { label: "PDF", fn: () => exportArtifactPdf(name, md) },
    ],
    [name, md, t],
  );

  return (
    <div className="relative inline-block">
      <button
        onClick={() => setOpen((o) => !o)}
        className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-3 py-1.5 text-xs text-zinc-300 transition hover:bg-white/5"
      >
        <Download size={13} /> {t.meeting.download}
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute left-0 z-20 mt-1 w-40 overflow-hidden rounded-lg border border-white/10 bg-zinc-900 shadow-xl">
            {items.map((it) => (
              <button
                key={it.label}
                onClick={() => {
                  setOpen(false);
                  void it.fn();
                }}
                className="block w-full px-3 py-2 text-left text-xs text-zinc-200 transition hover:bg-white/5"
              >
                {it.label}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
