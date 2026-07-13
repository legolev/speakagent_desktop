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
  Heart,
  Coins,
} from "lucide-react";
import { appInfo, systemInfo, isReady, llmReady, openUrl } from "../lib/api";
import { checkUpdate, installUpdate, type Update } from "../lib/update";
import Diagnostics from "../components/Diagnostics";
import { useT } from "../i18n";

const REPO = "https://github.com/legolev/speakagent_desktop";

// Донаты. Boosty — основной канал (РФ/СНГ, подписки + разовые); крипто — для зарубежных.
// Пусто → блок «Поддержать проект» скрыт.
const BOOSTY_URL: string = "https://boosty.to/utekov";
const CRYPTO: { label: string; addr: string }[] = [
  { label: "BTC", addr: "bc1qakjpcjwxkttxs9d3qlgp42dpacngzry2qurhfp" },
  { label: "TON (GRAM)", addr: "UQC2KkNMTefPUJEjGr0mMik2nNgOkP_NT-_qnmaBrZTHJ0LP" },
];

export default function AboutPage() {
  const t = useT();
  const { data: app } = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });
  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });
  const { data: asrReady } = useQuery({ queryKey: ["ready"], queryFn: isReady });
  const { data: llmR } = useQuery({ queryKey: ["llmReady"], queryFn: llmReady });

  const [eggs, setEggs] = useState(0);
  const [copied, setCopied] = useState("");

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
      <h1 className="text-2xl font-semibold tracking-tight">{t.about.title}</h1>

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
          <div className="text-sm text-zinc-400">{t.about.tagline}</div>
          <div className="mt-1 text-xs text-zinc-500">
            {t.about.version(app?.version ?? "…")}
          </div>
        </div>
      </div>
      {eggs >= 5 && (
        <div className="mt-2 text-xs text-amber-400/90">{t.about.easterEgg}</div>
      )}

      {/* Обновления */}
      <div className="glass mt-4 rounded-xl border border-white/5 p-4">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="text-sm font-medium text-zinc-200">{t.about.updates}</div>
            <div className="text-xs text-zinc-500">
              {checking
                ? t.about.checking
                : upd
                  ? t.about.updateAvailable(upd.version)
                  : checked
                    ? t.about.upToDate
                    : t.about.autoCheck}
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
                  <Loader2 size={15} className="animate-spin" /> {t.about.downloading(progress)}
                </>
              ) : (
                <>
                  <Download size={15} /> {t.about.updateBtn}
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
              {t.about.checkBtn}
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
          label={t.about.sourceCode}
          value="github.com/legolev/speakagent_desktop"
          onClick={() => openUrl(REPO).catch(() => {})}
        />
        <InfoRow icon={ShieldCheck} label={t.about.license} value="Apache 2.0" />
        <InfoRow
          icon={HardDrive}
          label={t.about.hardware}
          value={
            sys
              ? `${t.common.cores(sys.physicalCores)} · ${t.common.ramGb(sys.ramTotalGb.toFixed(0))}`
              : "…"
          }
        />
        <InfoRow
          icon={sys?.llmAccel === "gpu" ? Zap : Cpu}
          label={t.about.accel}
          value={
            sys
              ? sys.llmAccel === "gpu"
                ? t.about.gpuAccel(sys.gpuName)
                : t.about.cpu
              : "…"
          }
        />
      </div>

      {/* Поддержать проект (появляется, когда заданы ссылки/адреса на донаты) */}
      {(BOOSTY_URL || CRYPTO.length > 0) && (
        <>
          <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
            {t.about.supportTitle}
          </h2>
          <p className="mt-1 text-sm text-zinc-400">{t.about.supportText}</p>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            {BOOSTY_URL && (
              <button
                onClick={() => openUrl(BOOSTY_URL).catch(() => {})}
                className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
              >
                <Heart size={15} /> Boosty
              </button>
            )}
            {CRYPTO.map((c) => (
              <button
                key={c.label}
                title={t.about.copyAddrTitle(c.label, c.addr)}
                onClick={() => {
                  navigator.clipboard.writeText(c.addr).catch(() => {});
                  setCopied(c.label);
                  setTimeout(() => setCopied(""), 1500);
                }}
                className="inline-flex items-center gap-2 rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-200 transition hover:bg-white/5"
              >
                {copied === c.label ? (
                  <Check size={15} className="text-emerald-400" />
                ) : (
                  <Coins size={15} />
                )}
                {copied === c.label ? t.common.copied : c.label}
              </button>
            ))}
          </div>
        </>
      )}

      {/* Состояние компонентов */}
      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        {t.about.componentsTitle}
      </h2>
      <div className="glass mt-3 divide-y divide-white/5 rounded-xl border border-white/5">
        <StatusRow
          label={t.about.asrLabel}
          ok={!!asrReady}
          okText={t.about.asrOk}
          badText={t.about.asrBad}
        />
        <StatusRow
          label={t.about.aiLabel}
          ok={!!llmR}
          okText={t.about.aiOk}
          badText={t.about.aiBad}
        />
        <StatusRow
          label={t.about.accelLabel}
          ok={sys?.llmAccel === "gpu"}
          okText={sys?.gpuName ?? t.about.gpuFallback}
          badText={t.about.cpuOnly}
          neutral
        />
      </div>

      {/* Диагностика и поддержка */}
      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        {t.about.diagnosticsTitle}
      </h2>
      <p className="mt-1 text-sm text-zinc-400">{t.about.diagnosticsIntro}</p>
      <Diagnostics />

      <p className="mt-8 text-center text-xs text-zinc-600">{t.about.madeWith}</p>
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
