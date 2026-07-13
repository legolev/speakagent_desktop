import { useEffect, useMemo, useRef, useState } from "react";
import { RotateCcw, Search, Sparkles, Loader2 } from "lucide-react";
import AudioPlayer from "./AudioPlayer";
import DiarizeRenderer from "./DiarizeRenderer";
import SpeakerRoster from "./SpeakerRoster";
import { ArtifactPanel, TAB_DEFS, tabLabel, kindTitle, type Tab } from "./MeetingResults";
import { parseReplicas, speakerNumbers, timeToSec } from "../lib/diarize";
import { highlightText, countMatches } from "../lib/highlight";
import { useJobs } from "../store/jobs";
import { useT } from "../i18n";
import { beautifyConfig, type ResultKind } from "../lib/api";

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
  /** Обработчик «Расшифровать заново» (родитель показывает подтверждение и уводит на хаб). */
  onRetranscribe?: () => void;
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
  onRetranscribe,
}: Props) {
  const t = useT();
  const [time, setTime] = useState(0);
  const seekRef = useRef<((sec: number) => void) | null>(null);
  const [tab, setTab] = useState<Tab>("text");
  const [protoStyle, setProtoStyle] = useState<"business" | "interview">("business");
  const [tq, setTq] = useState(""); // поиск по тексту транскрипта
  const [view, setView] = useState<"original" | "beautified">("original");
  const [beautifyOn, setBeautifyOn] = useState(false);

  const hydrateResults = useJobs((s) => s.hydrateResults);
  const retranscribe = useJobs((s) => s.retranscribe);
  const busy = useJobs((s) => s.jobs.some((j) => j.status === "running"));
  const beautified = useJobs((s) => (jobId ? s.beautified[jobId] : undefined));
  const beautify = useJobs((s) => s.beautify);
  const cancelBeautify = useJobs((s) => s.cancelBeautify);
  const hydrateBeautified = useJobs((s) => s.hydrateBeautified);
  const mergeSpeaker = useJobs((s) => s.mergeSpeaker);
  const reassignReplica = useJobs((s) => s.reassignReplica);

  // Показываемый текст: оригинал или «обработанный» (если он выбран и уже готов).
  const shownText = view === "beautified" && beautified?.text ? beautified.text : text;

  const matches = useMemo(() => countMatches(shownText, tq), [shownText, tq]);
  useEffect(() => {
    if (jobId) void hydrateResults(jobId);
  }, [jobId, hydrateResults]);
  useEffect(() => {
    beautifyConfig()
      .then((c) => setBeautifyOn(c.enabled))
      .catch(() => {});
  }, []);
  useEffect(() => {
    if (jobId && beautifyOn) void hydrateBeautified(jobId);
  }, [jobId, beautifyOn, hydrateBeautified]);

  const starts = useMemo(
    () => parseReplicas(shownText).map((r) => timeToSec(r.time)),
    [shownText],
  );
  const activeIndex = useMemo(() => {
    let idx = -1;
    for (let i = 0; i < starts.length; i++) {
      if (starts[i] <= time + 0.05) idx = i;
      else break;
    }
    return idx;
  }, [starts, time]);

  // Переключатель «Оригинал | Обработанный» показываем только на готовой записи (с плеером).
  const showBeautifyUI = !!jobId && beautifyOn && withPlayer;
  const beautifyBusy = beautified?.status === "running";

  // Прокручиваемый транскрипт (реплики со спикерами или сплошной текст) + поиск по нему.
  const transcript = (
    <>
      {showBeautifyUI && (
        <div className="flex shrink-0 items-center gap-2 border-b border-white/5 px-3 py-2">
          <div className="inline-flex overflow-hidden rounded-lg border border-white/10 text-xs">
            <button
              onClick={() => setView("original")}
              className={`px-3 py-1 transition ${
                view === "original"
                  ? "bg-amber-500/15 text-amber-300"
                  : "text-zinc-400 hover:bg-white/5"
              }`}
            >
              {t.beautify.original}
            </button>
            <button
              onClick={() => setView("beautified")}
              className={`border-l border-white/10 px-3 py-1 transition ${
                view === "beautified"
                  ? "bg-amber-500/15 text-amber-300"
                  : "text-zinc-400 hover:bg-white/5"
              }`}
            >
              {t.beautify.beautified}
            </button>
          </div>
          {view === "beautified" && beautifyBusy && (
            <div className="ml-auto flex items-center gap-2 text-xs text-zinc-400">
              <Loader2 size={13} className="animate-spin text-amber-400" />
              {t.beautify.processing(beautified?.done ?? 0, beautified?.total ?? 0)}
              <button
                onClick={() => jobId && cancelBeautify(jobId)}
                className="rounded border border-white/10 px-2 py-0.5 text-zinc-300 transition hover:bg-white/5"
              >
                {t.beautify.stop}
              </button>
            </div>
          )}
          {view === "beautified" && beautified?.text && !beautifyBusy && (
            <button
              onClick={() => jobId && beautify(jobId)}
              className="ml-auto rounded border border-white/10 px-2 py-0.5 text-xs text-zinc-400 transition hover:bg-white/5"
            >
              {t.beautify.reprocess}
            </button>
          )}
        </div>
      )}
      {diarize && jobId && onRename && view === "original" && (
        <SpeakerRoster
          text={text}
          names={names}
          onMerge={(from, into) => mergeSpeaker(jobId, from, into)}
        />
      )}
      <div className="flex shrink-0 items-center gap-2 border-b border-white/5 px-3 py-2">
        <div className="relative flex-1">
          <Search
            size={13}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-500"
          />
          <input
            value={tq}
            onChange={(e) => setTq(e.target.value)}
            placeholder={t.resultView.searchPlaceholder}
            className="w-full rounded-lg border border-white/10 bg-white/5 py-1.5 pl-8 pr-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-500/50"
          />
        </div>
        {tq.trim() && (
          <span className="shrink-0 text-xs text-zinc-500">
            {matches > 0 ? t.resultView.matches(matches) : t.resultView.notFound}
          </span>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {view === "beautified" && !beautified?.text ? (
          <div className="mx-auto max-w-md pt-8 text-center">
            {beautifyBusy ? (
              <>
                <Loader2 size={22} className="mx-auto mb-3 animate-spin text-amber-400" />
                <div className="text-sm text-zinc-300">
                  {t.beautify.processing(beautified?.done ?? 0, beautified?.total ?? 0)}
                </div>
                {beautified?.partial && (
                  <div className="mt-3 max-h-40 overflow-y-auto whitespace-pre-wrap rounded-lg border border-white/5 bg-black/20 p-2 text-left text-xs text-zinc-400">
                    {beautified.partial}
                  </div>
                )}
              </>
            ) : (
              <>
                <Sparkles size={22} className="mx-auto mb-3 text-amber-500" />
                <div className="text-sm font-medium text-zinc-200">{t.beautify.emptyTitle}</div>
                <p className="mt-1.5 text-xs leading-relaxed text-zinc-500">
                  {t.beautify.emptyHint}
                </p>
                <p className="mt-1.5 text-xs leading-relaxed text-amber-400/80">
                  {t.beautify.costNote}
                </p>
                {beautified?.status === "error" && (
                  <p className="mt-2 text-xs text-red-400">{beautified.error}</p>
                )}
                <button
                  onClick={() => jobId && beautify(jobId)}
                  className="mt-4 inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
                >
                  <Sparkles size={15} />
                  {beautified?.status === "error" ? t.beautify.retry : t.beautify.process}
                </button>
              </>
            )}
          </div>
        ) : diarize ? (
          <DiarizeRenderer
            text={shownText}
            names={names}
            onRename={onRename}
            activeIndex={withPlayer ? activeIndex : undefined}
            onSeek={withPlayer ? (sec) => seekRef.current?.(sec) : undefined}
            query={tq}
            speakers={jobId && view === "original" ? speakerNumbers(text) : undefined}
            onReassign={
              jobId && view === "original"
                ? (i, sp) => reassignReplica(jobId, i, sp)
                : undefined
            }
          />
        ) : (
          <div className="select-text whitespace-pre-wrap text-sm leading-relaxed text-zinc-200">
            {shownText ? highlightText(shownText, tq) : t.resultView.noSpeech}
          </div>
        )}
      </div>
    </>
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
                {s === "business" ? t.resultView.protoBusiness : t.resultView.protoInterview}
              </button>
            ))}
          </div>
        )}
        <div className="min-h-0 flex-1">
          <ArtifactPanel
            jobId={jobId}
            kind={(tab === "protocol" ? protoStyle : tab) as ResultKind}
            exportName={`${name ?? t.resultView.exportFallbackName} — ${kindTitle(
              t,
              tab === "protocol" ? protoStyle : tab,
            )}`}
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
              {t.resultView.clickReplica}
            </div>
          )}
        </div>
        {jobId && (
          <button
            onClick={() => (onRetranscribe ? onRetranscribe() : retranscribe(jobId))}
            disabled={busy}
            title={t.resultView.retranscribeTitle}
            className="flex items-center justify-center gap-2 rounded-xl border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm font-medium text-amber-300 transition hover:bg-amber-500/20 disabled:cursor-default disabled:opacity-40"
          >
            <RotateCcw size={15} /> {busy ? t.resultView.transcribing : t.resultView.retranscribe}
          </button>
        )}
        {jobId && (
          <div className="flex flex-col gap-0.5 rounded-xl border border-white/10 bg-black/20 p-2">
            {TAB_DEFS.map((def) => {
              const Icon = def.icon;
              const active = tab === def.id;
              return (
                <button
                  key={def.id}
                  onClick={() => setTab(def.id)}
                  className={`flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition ${
                    active
                      ? "bg-amber-500/15 text-amber-300"
                      : "text-zinc-400 hover:bg-white/5 hover:text-zinc-100"
                  }`}
                >
                  <Icon size={15} /> {tabLabel(t, def.id)}
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
