import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Channel } from "@tauri-apps/api/core";
import { AudioLines, WifiOff, Users, Lock, Check } from "lucide-react";
import {
  listModels,
  downloadModel,
  setActiveModel,
  type ModelInfo,
  type DlProgress,
} from "../lib/api";
import { useUi } from "../store/ui";

export default function Onboarding() {
  const qc = useQueryClient();
  const closeSetup = useUi((s) => s.closeSetup);
  const { data: models } = useQuery({ queryKey: ["models"], queryFn: listModels });

  const asr = (models ?? []).filter((m) => m.kind === "asr");
  const [selected, setSelected] = useState("gigaam");
  const [pct, setPct] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function go() {
    const m = asr.find((x) => x.id === selected);
    if (!m) return;
    setError(null);
    setPct(0);
    try {
      if (!m.installed) {
        const ch = new Channel<DlProgress>();
        ch.onmessage = (d) => setPct(d.total > 0 ? d.done / d.total : 0);
        await downloadModel(selected, ch);
      }
      await setActiveModel(selected);
      await qc.invalidateQueries();
      closeSetup();
    } catch (e) {
      setError(String(e));
      setPct(null);
    }
  }

  const busy = pct !== null;

  return (
    <div className="anim-fade fixed inset-0 z-50 flex items-center justify-center bg-zinc-950/85 p-6 backdrop-blur-md">
      <div className="anim-in glass w-full max-w-lg rounded-2xl border border-white/10 p-8">
        <div className="flex items-center gap-3">
          <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-amber-500/15">
            <AudioLines className="text-amber-500" size={22} />
          </div>
          <div>
            <h1 className="text-lg font-semibold tracking-tight">Добро пожаловать в SpeakAgent</h1>
            <p className="text-sm text-zinc-400">Расшифровка речи прямо на вашем компьютере</p>
          </div>
        </div>

        <div className="mt-5 grid grid-cols-3 gap-2 text-center">
          <Feature icon={WifiOff} label="Без интернета" />
          <Feature icon={Users} label="Различает голоса" />
          <Feature icon={Lock} label="Ничего не уходит в сеть" />
        </div>

        <div className="mt-6">
          <div className="text-sm font-medium">Выберите язык распознавания</div>
          <p className="mt-1 text-xs text-zinc-500">
            Модель скачается один раз (размер указан ниже) — плюс конвертер аудио (~98 МБ).
            Остальное уже внутри приложения. Позже можно сменить в настройках.
          </p>

          <div className="mt-3 flex flex-col gap-1.5">
            {asr.map((m) => (
              <ModelRow
                key={m.id}
                model={m}
                active={selected === m.id}
                recommended={m.id === "gigaam"}
                disabled={busy}
                onClick={() => !busy && setSelected(m.id)}
              />
            ))}
          </div>
        </div>

        {busy ? (
          <div className="mt-6">
            <div className="h-2 w-full overflow-hidden rounded-full bg-white/10">
              <div
                className="h-full rounded-full bg-amber-500 transition-[width]"
                style={{ width: `${Math.round((pct ?? 0) * 100)}%` }}
              />
            </div>
            <div className="mt-2 text-center text-xs text-zinc-400">
              Скачиваю модель… {Math.round((pct ?? 0) * 100)}%
            </div>
          </div>
        ) : (
          <div className="mt-6 flex items-center justify-between">
            <button
              onClick={closeSetup}
              className="text-xs text-zinc-500 transition hover:text-zinc-300"
            >
              Позже
            </button>
            <button
              onClick={go}
              className="rounded-lg bg-amber-500 px-5 py-2.5 font-medium text-zinc-950 transition hover:bg-amber-400"
            >
              Начать
            </button>
          </div>
        )}

        {error && <div className="mt-3 text-center text-xs text-red-400">{error}</div>}
      </div>
    </div>
  );
}

function Feature({
  icon: Icon,
  label,
}: {
  icon: React.ComponentType<{ size?: number; className?: string }>;
  label: string;
}) {
  return (
    <div className="rounded-lg border border-white/5 bg-white/5 p-3">
      <Icon size={18} className="mx-auto text-amber-500/80" />
      <div className="mt-1.5 text-xs text-zinc-400">{label}</div>
    </div>
  );
}

function ModelRow({
  model,
  active,
  recommended,
  disabled,
  onClick,
}: {
  model: ModelInfo;
  active: boolean;
  recommended: boolean;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`flex items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition ${
        active
          ? "border-amber-500/50 bg-amber-500/10"
          : "border-white/10 hover:bg-white/5"
      } disabled:cursor-default`}
    >
      <span
        className={`flex h-4 w-4 shrink-0 items-center justify-center rounded-full border ${
          active ? "border-amber-500" : "border-zinc-600"
        }`}
      >
        {active && <span className="h-2 w-2 rounded-full bg-amber-500" />}
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-2">
          <span className="text-sm text-zinc-100">{model.name}</span>
          {recommended && (
            <span className="rounded bg-amber-500/20 px-1.5 py-0.5 text-[10px] font-medium text-amber-300">
              Рекомендуем
            </span>
          )}
        </span>
        <span className="block text-xs text-zinc-500">
          {model.lang} · {model.sizeMb} МБ
        </span>
      </span>
      {model.installed && <Check size={15} className="shrink-0 text-emerald-400" />}
    </button>
  );
}
