import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Cpu, MemoryStick, Globe } from "lucide-react";
import { systemInfo, resourceUsage, type Usage } from "../lib/api";
import { useJobs } from "../store/jobs";
import { fmtEta, estimateTranscribeSec } from "../lib/perf";
import { useT, useLang } from "../i18n";

export default function StatusBar() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const setLang = useLang((s) => s.setLang);
  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });
  const jobs = useJobs((s) => s.jobs);
  const results = useJobs((s) => s.results);
  const running = jobs.find((j) => j.status === "running");
  // идёт ли генерация «Итогов» (в любой записи)
  const genState = Object.values(results)
    .flatMap((byKind) => Object.values(byKind))
    .find((r) => r?.status === "running");

  const [usage, setUsage] = useState<Usage | null>(null);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!running) {
      setUsage(null);
      return;
    }
    let alive = true;
    const tick = async () => {
      setNow(Date.now());
      try {
        const u = await resourceUsage();
        if (alive) setUsage(u);
      } catch {
        /* ignore */
      }
    };
    void tick();
    const t = setInterval(tick, 1800);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [running?.id]);

  // «Осталось»: когда идёт распознавание и есть прогресс по кускам — считаем по
  // РЕАЛЬНОЙ скорости (elapsed/done × remaining); до того — по оценке железа.
  let remaining: number | null = null;
  if (running && sys) {
    if (running.stage === "transcribing" && running.done > 1 && running.total > 0) {
      const stageElapsed = (now - running.stageStartedAt) / 1000;
      remaining = Math.max(0, (stageElapsed / running.done) * (running.total - running.done));
    } else if (running.durationSec) {
      const total = estimateTranscribeSec(sys, running.durationSec, running.diarize);
      remaining = Math.max(0, total - (now - running.startedAt) / 1000);
    }
  }

  return (
    <div className="glass flex h-8 shrink-0 items-center justify-between border-t border-white/5 px-4 text-xs text-zinc-500">
      <div>
        {sys && (
          <span>
            {t.common.cores(sys.physicalCores)} · {t.common.ramGb(sys.ramTotalGb.toFixed(0))}
            {sys.gpuName ? ` · ${sys.gpuName}` : ""}
          </span>
        )}
      </div>
      <div className="flex items-center gap-3">
        {genState && (
          <span className="text-amber-400/80">
            {genState.stage === "reading"
              ? t.statusBar.summarizingReading(genState.done, genState.total)
              : t.statusBar.summarizing}
          </span>
        )}
        {running && usage && (
          <>
            <span className="inline-flex items-center gap-1">
              <Cpu size={12} /> {Math.round(usage.cpuPct)}%
            </span>
            <span className="inline-flex items-center gap-1">
              <MemoryStick size={12} /> {Math.round(usage.ramUsedPct)}%
            </span>
          </>
        )}
        {running && remaining !== null && (
          <span className="text-amber-400/80">{t.statusBar.remaining(fmtEta(remaining))}</span>
        )}
        <button
          onClick={() => setLang(lang === "ru" ? "en" : "ru")}
          title={t.statusBar.switchLang}
          className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-zinc-400 transition hover:bg-white/5 hover:text-zinc-200"
        >
          <Globe size={12} /> {lang.toUpperCase()}
        </button>
      </div>
    </div>
  );
}
