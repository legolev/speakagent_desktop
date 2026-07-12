import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Github,
  Zap,
  Check,
  ShieldCheck,
  HardDrive,
  Cpu,
  ExternalLink,
  RefreshCw,
  Loader2,
  Download,
} from "lucide-react";
import { appInfo, systemInfo, isReady, llmReady, openUrl } from "../lib/api";
import { checkUpdate, installUpdate, type Update } from "../lib/update";
import Diagnostics from "../components/Diagnostics";

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

  // ── Обновления ──
  const [upd, setUpd] = useState<Update | null>(null);
  const [checking, setChecking] = useState(false);
  const [checked, setChecked] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [updErr, setUpdErr] = useState("");

  async function doCheck(manual: boolean) {
    setChecking(true);
    setUpdErr("");
    try {
      setUpd(await checkUpdate());
      setChecked(true);
    } catch (e) {
      if (manual) setUpdErr(String(e));
    } finally {
      setChecking(false);
    }
  }

  // Тихая автопроверка при открытии страницы.
  useEffect(() => {
    void doCheck(false);
  }, []);

  async function doInstall() {
    if (!upd) return;
    setDownloading(true);
    setProgress(0);
    setUpdErr("");
    try {
      await installUpdate(upd, setProgress); // внутри — перезапуск приложения
    } catch (e) {
      setUpdErr(String(e));
      setDownloading(false);
    }
  }

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
          <div className="mt-1 text-xs text-zinc-500">Версия {app?.version ?? "…"}</div>
        </div>
      </div>
      {eggs >= 5 && (
        <div className="mt-2 text-xs text-amber-400/90">
          🥚 Все ваши записи так и не покинули этот компьютер. Ни байта. Спасибо, что
          выбираете приватность!
        </div>
      )}

      {/* Обновления */}
      <div className="glass mt-4 rounded-xl border border-white/5 p-4">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="text-sm font-medium text-zinc-200">Обновления</div>
            <div className="text-xs text-zinc-500">
              {checking
                ? "Проверяю наличие обновлений…"
                : upd
                  ? `Доступна новая версия ${upd.version}`
                  : checked
                    ? "У вас последняя версия"
                    : "Автоматическая проверка обновлений"}
            </div>
          </div>
          {upd ? (
            <button
              onClick={doInstall}
              disabled={downloading}
              className="inline-flex shrink-0 items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400 disabled:opacity-60"
            >
              {downloading ? (
                <>
                  <Loader2 size={15} className="animate-spin" /> Загрузка {progress}%
                </>
              ) : (
                <>
                  <Download size={15} /> Обновить
                </>
              )}
            </button>
          ) : (
            <button
              onClick={() => doCheck(true)}
              disabled={checking}
              className="inline-flex shrink-0 items-center gap-2 rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-200 transition hover:bg-white/5 disabled:opacity-60"
            >
              {checking ? (
                <Loader2 size={15} className="animate-spin" />
              ) : (
                <RefreshCw size={15} />
              )}
              Проверить
            </button>
          )}
        </div>
        {upd?.body && (
          <p className="mt-3 whitespace-pre-line border-t border-white/5 pt-3 text-xs leading-relaxed text-zinc-400">
            {upd.body}
          </p>
        )}
        {downloading && (
          <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-white/10">
            <div
              className="h-full rounded-full bg-amber-500 transition-all"
              style={{ width: `${progress}%` }}
            />
          </div>
        )}
        {updErr && <div className="mt-2 text-xs text-red-400">{updErr}</div>}
      </div>

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

      {/* Диагностика и поддержка */}
      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        Диагностика и поддержка
      </h2>
      <p className="mt-1 text-sm text-zinc-400">
        Уникальный ID этого компьютера и служебная информация — пригодятся, если нужно задать
        вопрос или сообщить о проблеме.
      </p>
      <Diagnostics />

      <p className="mt-8 text-center text-xs text-zinc-600">
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
