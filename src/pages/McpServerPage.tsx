import { useEffect, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Server,
  Power,
  PowerOff,
  Copy,
  Check,
  Loader2,
  Wrench,
  ShieldCheck,
  ExternalLink,
} from "lucide-react";
import {
  mcpStatus,
  mcpConfig,
  mcpStart,
  mcpStop,
  setMcpConfig,
  openUrl,
  type McpConfig,
} from "../lib/api";
import { useT } from "../i18n";

export default function McpServerPage() {
  const t = useT();
  const toolHints: Record<string, string> = t.mcp.tools;
  const { data: status, refetch } = useQuery({ queryKey: ["mcpStatus"], queryFn: mcpStatus });
  const { data: cfg0 } = useQuery({ queryKey: ["mcpConfig"], queryFn: mcpConfig });

  const [port, setPort] = useState(8722);
  const [token, setToken] = useState("");
  const [autostart, setAutostart] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (cfg0) {
      setPort(cfg0.port);
      setToken(cfg0.token);
      setAutostart(cfg0.autostart);
    }
  }, [cfg0]);

  const running = status?.running ?? false;
  const url = status?.url ?? `http://127.0.0.1:${port}/mcp`;

  async function apply(enabled: boolean) {
    setBusy(true);
    setErr("");
    try {
      const cfg: McpConfig = { enabled, port, token, autostart };
      await setMcpConfig(cfg);
      await refetch();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function toggle() {
    setBusy(true);
    setErr("");
    try {
      if (running) await mcpStop();
      else await mcpStart();
      await refetch();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function saveSettings() {
    await apply(running); // сохранить порт/токен/автозапуск, сохранив текущее вкл/выкл
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  }

  return (
    <div className="mx-auto max-w-3xl p-8">
      <div className="flex items-center gap-3">
        <Server size={22} className="text-amber-500" />
        <h1 className="text-2xl font-semibold tracking-tight">{t.mcp.title}</h1>
      </div>
      <p className="mt-2 text-sm leading-relaxed text-zinc-400">
        {t.mcp.intro}
      </p>

      {/* Статус + вкл/выкл */}
      <div className="glass mt-5 flex items-center justify-between gap-4 rounded-xl border border-white/5 p-4">
        <div className="flex items-center gap-3">
          <span className="relative flex h-2.5 w-2.5">
            {running && (
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-500 opacity-70" />
            )}
            <span
              className={`relative inline-flex h-2.5 w-2.5 rounded-full ${
                running ? "bg-emerald-500" : "bg-zinc-600"
              }`}
            />
          </span>
          <div>
            <div className="text-sm font-medium text-zinc-100">
              {running ? t.mcp.running : t.mcp.stopped}
            </div>
            {running && <div className="font-mono text-xs text-zinc-500">{url}</div>}
          </div>
        </div>
        <button
          onClick={toggle}
          disabled={busy}
          className={`inline-flex items-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition disabled:opacity-50 ${
            running
              ? "border border-white/10 text-zinc-200 hover:bg-white/5"
              : "bg-amber-500 text-zinc-950 hover:bg-amber-400"
          }`}
        >
          {busy ? (
            <Loader2 size={15} className="animate-spin" />
          ) : running ? (
            <PowerOff size={15} />
          ) : (
            <Power size={15} />
          )}
          {running ? t.common.stop : t.mcp.start}
        </button>
      </div>

      {err && <div className="mt-3 text-xs text-red-400">{err}</div>}

      {/* Подключение */}
      <div className="mt-6">
        <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">{t.mcp.connectHeading}</h2>
        <p className="mt-2 text-xs text-zinc-500">{t.mcp.quickAdd}</p>
        <ClientButtons url={url} hasToken={status?.hasToken ?? false} token={token} />
        <p className="mt-4 text-xs text-zinc-500">
          {t.mcp.claudeCodeHint}
        </p>
        <CopyBlock text={`claude mcp add --transport http speakagent ${url}`} />
        <p className="mt-3 text-xs text-zinc-500">{t.mcp.configHint}</p>
        <CopyBlock
          text={JSON.stringify(
            {
              mcpServers: {
                speakagent: {
                  url,
                  ...(status?.hasToken ? { headers: { Authorization: t.mcp.bearerToken } } : {}),
                },
              },
            },
            null,
            2,
          )}
        />
      </div>

      {/* Инструменты */}
      <div className="mt-6">
        <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">{t.mcp.toolsHeading}</h2>
        <div className="glass mt-3 grid grid-cols-1 gap-1.5 rounded-xl border border-white/5 p-4 sm:grid-cols-2">
          {(status?.tools ?? []).map((tool) => (
            <div key={tool} className="flex items-start gap-2 text-sm">
              <Wrench size={14} className="mt-0.5 shrink-0 text-amber-500/80" />
              <div>
                <span className="font-mono text-zinc-200">{tool}</span>
                {toolHints[tool] && <span className="ml-1 text-xs text-zinc-500">— {toolHints[tool]}</span>}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Настройки */}
      <div className="glass mt-6 space-y-3 rounded-xl border border-white/5 p-4">
        <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">{t.mcp.settingsHeading}</h2>
        <label className="block">
          <span className="mb-1 block text-xs text-zinc-400">{t.mcp.port}</span>
          <input
            type="number"
            value={port}
            min={1024}
            max={65535}
            onChange={(e) => setPort(Number(e.target.value) || 8722)}
            className="w-36 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-zinc-100 outline-none focus:border-amber-500/50"
          />
        </label>
        <label className="block">
          <span className="mb-1 flex items-center gap-1.5 text-xs text-zinc-400">
            <ShieldCheck size={13} /> {t.mcp.token}
          </span>
          <input
            type="text"
            value={token}
            placeholder={t.mcp.tokenPlaceholder}
            spellCheck={false}
            autoComplete="off"
            onChange={(e) => setToken(e.target.value)}
            className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-500/50"
          />
        </label>
        <label className="flex cursor-pointer items-center gap-2 text-sm text-zinc-300">
          <input
            type="checkbox"
            checked={autostart}
            onChange={(e) => setAutostart(e.target.checked)}
            className="h-4 w-4 accent-amber-500"
          />
          {t.mcp.autostart}
        </label>
        <div className="pt-1">
          <button
            onClick={saveSettings}
            disabled={busy}
            className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400 disabled:opacity-50"
          >
            {saved ? <Check size={15} /> : null} {saved ? t.common.saved : t.mcp.saveApply}
          </button>
        </div>
      </div>
    </div>
  );
}

function ClientButtons({
  url,
  hasToken,
  token,
}: {
  url: string;
  hasToken: boolean;
  token: string;
}) {
  const [copied, setCopied] = useState<string | null>(null);
  const copy = (id: string, text: string) =>
    navigator.clipboard.writeText(text).then(() => {
      setCopied(id);
      setTimeout(() => setCopied((c) => (c === id ? null : c)), 1500);
    });

  // Если задан токен — вшиваем заголовок авторизации прямо в deep-link, чтобы
  // установка «в один клик» сразу давала рабочее подключение.
  const headers =
    hasToken && token.trim() ? { Authorization: `Bearer ${token.trim()}` } : undefined;
  // Cursor: config — стандартный base64 внутреннего объекта сервера ({url,...}).
  const cursorLink = `cursor://anysphere.cursor-deeplink/mcp/install?name=speakagent&config=${btoa(
    JSON.stringify({ url, ...(headers ? { headers } : {}) }),
  )}`;
  // VS Code: URL-encoded JSON с name + type:"http" (обязателен для HTTP-сервера).
  const vscodeLink = `vscode:mcp/install?${encodeURIComponent(
    JSON.stringify({ name: "speakagent", type: "http", url, ...(headers ? { headers } : {}) }),
  )}`;
  const claudeCmd = `claude mcp add --transport http speakagent ${url}`;
  const codexToml = `# ~/.codex/config.toml\n[mcp_servers.speakagent]\nurl = "${url}"`;

  const ok = (id: string) =>
    copied === id ? <Check size={14} className="text-emerald-400" /> : null;

  return (
    <div className="mt-2 flex flex-wrap gap-2">
      <Btn onClick={() => openUrl(cursorLink).catch(() => {})} icon={<ExternalLink size={14} />}>
        Cursor
      </Btn>
      <Btn onClick={() => openUrl(vscodeLink).catch(() => {})} icon={<ExternalLink size={14} />}>
        VS Code
      </Btn>
      <Btn onClick={() => copy("claude", claudeCmd)} icon={ok("claude") ?? <Copy size={14} />}>
        Claude Code
      </Btn>
      <Btn onClick={() => copy("codex", codexToml)} icon={ok("codex") ?? <Copy size={14} />}>
        Codex / ChatGPT
      </Btn>
    </div>
  );
}

function Btn({
  onClick,
  icon,
  children,
}: {
  onClick: () => void;
  icon: ReactNode;
  children: string;
}) {
  return (
    <button
      onClick={onClick}
      className="inline-flex items-center gap-1.5 rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-xs text-zinc-200 transition hover:bg-white/10"
    >
      {icon} {children}
    </button>
  );
}

function CopyBlock({ text }: { text: string }) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  return (
    <div className="group relative mt-1.5">
      <pre className="overflow-x-auto rounded-lg border border-white/5 bg-black/30 p-3 pr-10 font-mono text-xs leading-relaxed text-zinc-300">
        {text}
      </pre>
      <button
        onClick={() => {
          navigator.clipboard.writeText(text).then(() => {
            setCopied(true);
            setTimeout(() => setCopied(false), 1500);
          });
        }}
        title={t.common.copy}
        className="absolute right-2 top-2 rounded-md p-1.5 text-zinc-500 transition hover:bg-white/10 hover:text-zinc-200"
      >
        {copied ? <Check size={14} className="text-emerald-400" /> : <Copy size={14} />}
      </button>
    </div>
  );
}
