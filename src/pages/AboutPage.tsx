import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Github,
  Zap,
  Check,
  ShieldCheck,
  HardDrive,
  Cpu,
  ExternalLink,
} from "lucide-react";
import { appInfo, systemInfo, isReady, llmReady, openUrl } from "../lib/api";

const REPO = "https://github.com/legolev/speakagent_desktop";

export default function AboutPage() {
  const { data: app } = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });
  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });
  const { data: asrReady } = useQuery({ queryKey: ["ready"], queryFn: isReady });
  const { data: llmR } = useQuery({ queryKey: ["llmReady"], queryFn: llmReady });

  const [eggs, setEggs] = useState(0);

  return (
    <div className="mx-auto max-w-3xl p-8">
      <h1 className="text-2xl font-semibold tracking-tight">О приложении</h1>

      {/* Шапка */}
      <div className="glass mt-6 flex items-center gap-4 rounded-2xl border border-white/5 p-6">
        <button
          onClick={() => setEggs((e) => e + 1)}
          className="select-none text-4xl leading-none transition hover:scale-110"
          title="🎙️"
        >
          🎙️
        </button>
        <div className="min-w-0">
          <div className="text-xl font-semibold">
            Speak<span className="text-amber-500">Agent</span> Desktop
          </div>
          <div className="text-sm text-zinc-400">
            Расшифровка и диаризация речи — офлайн, на вашем компьютере
          </div>
          <div className="mt-1 text-xs text-zinc-500">
            Версия {app?.version ?? "…"} · Apple Silicon · macOS
          </div>
        </div>
      </div>
      {eggs >= 5 && (
        <div className="mt-2 text-xs text-amber-400/90">
          🥚 Все ваши записи так и не покинули этот компьютер. Ни байта. Спасибо, что
          выбираете приватность!
        </div>
      )}

      {/* Полезное */}
      <div className="mt-4 grid gap-2 sm:grid-cols-2">
        <InfoRow
          icon={Github}
          label="Исходный код"
          value="github.com/legolev/speakagent_desktop"
          onClick={() => openUrl(REPO).catch(() => {})}
        />
        <InfoRow icon={ShieldCheck} label="Лицензия" value="PolyForm Noncommercial 1.0.0" />
        <InfoRow
          icon={HardDrive}
          label="Железо"
          value={sys ? `${sys.physicalCores} ядер · ${sys.ramTotalGb.toFixed(0)} ГБ ОЗУ` : "…"}
        />
        <InfoRow
          icon={sys?.llmAccel === "gpu" ? Zap : Cpu}
          label="Ускорение"
          value={
            sys
              ? sys.llmAccel === "gpu"
                ? `видеокарта — ${sys.gpuName}`
                : "процессор"
              : "…"
          }
        />
      </div>

      {/* Состояние компонентов */}
      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        Состояние компонентов
      </h2>
      <div className="glass mt-3 divide-y divide-white/5 rounded-xl border border-white/5">
        <StatusRow
          label="Распознавание речи"
          ok={!!asrReady}
          okText="готово к работе"
          badText="нужно скачать модель"
        />
        <StatusRow
          label="ИИ-функции (итоги встреч)"
          ok={!!llmR}
          okText="готово"
          badText="не настроено"
        />
        <StatusRow
          label="Ускорение вычислений"
          ok={sys?.llmAccel === "gpu"}
          okText={sys?.gpuName ?? "видеокарта"}
          badText="только процессор"
          neutral
        />
      </div>

      <p className="mt-6 text-center text-xs text-zinc-600">
        Сделано с ❤️ для тех, кому важна приватность записей.
      </p>
    </div>
  );
}

function InfoRow({
  icon: Icon,
  label,
  value,
  onClick,
}: {
  icon: React.ComponentType<{ size?: number; className?: string }>;
  label: string;
  value: string;
  onClick?: () => void;
}) {
  const inner = (
    <>
      <Icon size={16} className="mt-0.5 shrink-0 text-zinc-500" />
      <div className="min-w-0">
        <div className="text-xs text-zinc-500">{label}</div>
        <div className="flex items-center gap-1 truncate text-sm text-zinc-200">
          {value}
          {onClick && <ExternalLink size={12} className="shrink-0 text-zinc-500" />}
        </div>
      </div>
    </>
  );
  const cls =
    "flex items-start gap-3 rounded-xl border border-white/5 bg-white/[0.02] p-4 text-left";
  return onClick ? (
    <button onClick={onClick} className={`${cls} transition hover:bg-white/5`}>
      {inner}
    </button>
  ) : (
    <div className={cls}>{inner}</div>
  );
}

function StatusRow({
  label,
  ok,
  okText,
  badText,
  neutral,
}: {
  label: string;
  ok?: boolean;
  okText: string;
  badText: string;
  neutral?: boolean;
}) {
  const good = !!ok;
  const dot = good ? "bg-emerald-500" : neutral ? "bg-zinc-500" : "bg-amber-500";
  const text = good ? "text-emerald-400/90" : neutral ? "text-zinc-400" : "text-amber-400/90";
  return (
    <div className="flex items-center justify-between px-4 py-3">
      <span className="text-sm text-zinc-300">{label}</span>
      <span className={`inline-flex items-center gap-1.5 text-xs ${text}`}>
        {good && <Check size={13} />}
        <span className={`h-1.5 w-1.5 rounded-full ${dot} ${good ? "hidden" : ""}`} />
        {good ? okText : badText}
      </span>
    </div>
  );
}
