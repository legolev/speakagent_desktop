import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Channel } from "@tauri-apps/api/core";
import { Check, Download } from "lucide-react";
import {
  listModels,
  activeModel,
  setActiveModel,
  downloadModel,
  systemInfo,
  type ModelInfo,
  type SystemInfo,
  type DlProgress,
} from "../lib/api";
import { useT } from "../i18n";

/** Насколько модель подходит под железо этого компьютера. */
type Fit = "good" | "ok" | "heavy";

/** Оценка пригодности модели: память (модель в RAM + запас), тяжесть на CPU и общий
 *  класс машины. Наличие ускорителя (Apple Silicon / дискретная видеокарта) считаем
 *  признаком сильной машины — тяжёлые модели на ней комфортнее. */
function modelFit(
  m: ModelInfo,
  sys: SystemInfo,
  t: ReturnType<typeof useT>,
): { level: Fit; note: string } {
  // Сколько примерно занимает в памяти (ГБ) — по факту распаковки, не по размеру архива.
  const ramNeedGb: Record<string, number> = {
    gigaam: 0.7,
    parakeet: 1.3,
    "whisper-small": 1.3,
    "whisper-turbo": 2.6,
  };
  const need = ramNeedGb[m.id] ?? Math.max(0.7, m.sizeMb / 1024);

  // Не хватает памяти под модель + рабочий запас → тяжело при любом раскладе.
  if (sys.ramTotalGb < need + 1.5) {
    return { level: "heavy", note: t.modelSelector.noteLowRam };
  }

  // Сильная машина: есть ускоритель (GPU/Apple Silicon) ИЛИ быстрый многоядерный CPU.
  const strong =
    sys.llmAccel === "gpu" || !!sys.gpuName || (sys.physicalCores >= 8 && sys.speed !== "slow");

  if (m.id === "whisper-turbo") {
    // large-v3-turbo тяжёлый: на сильной машине приемлемо, на слабой — очень медленно.
    return strong
      ? { level: "ok", note: t.modelSelector.noteMostAccurateSlower }
      : { level: "heavy", note: t.modelSelector.noteVerySlowCpu };
  }
  if (m.id === "whisper-small" || m.id === "parakeet") {
    if (sys.speed === "slow" && !strong) return { level: "heavy", note: t.modelSelector.noteWillBeSlow };
    return strong
      ? { level: "good", note: "" }
      : { level: "ok", note: t.modelSelector.noteSlightlySlower };
  }
  // GigaAM — лёгкая CTC-модель, идёт почти на любом железе.
  return { level: "good", note: "" };
}

const FIT_UI: Record<Fit, { dot: string; text: string }> = {
  good: { dot: "bg-emerald-500", text: "text-emerald-400/90" },
  ok: { dot: "bg-amber-500", text: "text-amber-400/90" },
  heavy: { dot: "bg-red-500", text: "text-red-400/90" },
};

function FitBadge({ m, sys }: { m: ModelInfo; sys?: SystemInfo }) {
  const t = useT();
  if (!sys) return null;
  const { level, note } = modelFit(m, sys, t);
  const ui = FIT_UI[level];
  const fitLabel: Record<Fit, string> = {
    good: t.modelSelector.fitGood,
    ok: t.modelSelector.fitOk,
    heavy: t.modelSelector.fitHeavy,
  };
  const hw = `${t.common.cores(sys.physicalCores)}, ${t.common.ramGb(
    sys.ramTotalGb.toFixed(0),
  )}${sys.gpuName ? `, ${sys.gpuName}` : ""}`;
  return (
    <span
      className={`mt-1 inline-flex items-center gap-1.5 text-[11px] ${ui.text}`}
      title={t.modelSelector.hwEstimateTitle(hw)}
    >
      <span className={`h-1.5 w-1.5 rounded-full ${ui.dot}`} />
      {fitLabel[level]}
      {note && <span className="text-zinc-500">· {note}</span>}
    </span>
  );
}

export default function ModelSelector() {
  const t = useT();
  const { data: models, refetch } = useQuery({ queryKey: ["models"], queryFn: listModels });
  const { data: active, refetch: refetchActive } = useQuery({
    queryKey: ["activeModel"],
    queryFn: activeModel,
  });
  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });
  const [progress, setProgress] = useState<Record<string, number>>({});
  const [error, setError] = useState<Record<string, string>>({});

  const asr = (models ?? []).filter((m) => m.kind === "asr");

  async function choose(m: ModelInfo) {
    if (m.installed) {
      await setActiveModel(m.id);
      await refetchActive();
      return;
    }
    setError((e) => ({ ...e, [m.id]: "" }));
    setProgress((p) => ({ ...p, [m.id]: 0 }));
    const ch = new Channel<DlProgress>();
    ch.onmessage = (d) =>
      setProgress((p) => ({ ...p, [m.id]: d.total > 0 ? d.done / d.total : 0 }));
    try {
      await downloadModel(m.id, ch);
      await refetch();
      await setActiveModel(m.id);
      await refetchActive();
    } catch (e) {
      setError((er) => ({ ...er, [m.id]: String(e) }));
    } finally {
      setProgress((p) => {
        const n = { ...p };
        delete n[m.id];
        return n;
      });
    }
  }

  return (
    <div className="glass mt-4 rounded-xl border border-white/5 p-2">
      {asr.map((m) => {
        const isActive = active === m.id;
        const pct = progress[m.id];
        const busy = pct !== undefined;
        return (
          <button
            key={m.id}
            onClick={() => !busy && choose(m)}
            disabled={busy}
            className={`flex w-full items-center gap-3 rounded-lg px-3 py-3 text-left transition ${
              isActive ? "bg-amber-500/10" : "hover:bg-white/5"
            } disabled:cursor-default`}
          >
            <span
              className={`flex h-4 w-4 shrink-0 items-center justify-center rounded-full border ${
                isActive ? "border-amber-500" : "border-zinc-600"
              }`}
            >
              {isActive && <span className="h-2 w-2 rounded-full bg-amber-500" />}
            </span>

            <span className="min-w-0 flex-1">
              <span className="block text-sm text-zinc-200">
                {t.models[m.id]?.name ?? m.name}
              </span>
              <span className="block text-xs text-zinc-500">
                {t.models[m.id]?.lang ?? m.lang} · {t.common.mb(m.sizeMb)}
                {!m.installed && ` · ${t.common.notDownloaded}`}
              </span>
              <FitBadge m={m} sys={sys} />
              {error[m.id] && <span className="block text-xs text-red-400">{error[m.id]}</span>}
            </span>

            {busy ? (
              <span className="text-xs text-amber-400">{Math.round(pct * 100)}%</span>
            ) : m.installed ? (
              isActive ? (
                <Check size={16} className="text-amber-400" />
              ) : null
            ) : (
              <Download size={15} className="text-zinc-400" />
            )}
          </button>
        );
      })}
    </div>
  );
}
