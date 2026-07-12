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

const TOOL_HINTS: Record<string, string> = {
  status: "версия, активная модель, готовность",
  transcribe: "распознать файл → текст",
  diarize: "распознать с разделением по говорящим",
  protocol: "протокол/саммари встречи (LLM)",
  todo: "список задач из записи (LLM)",
  summarize: "итог по записи из истории",
  list_jobs: "список записей истории",
  get_transcript: "текст записи по id",
};

export default function McpServerPage() {
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
        <h1 className="text-2xl font-semibold tracking-tight">MCP-сервер</h1>
      </div>
      <p className="mt-2 text-sm leading-relaxed text-zinc-400">
        Локальный сервер Model Context Protocol. Любой ИИ/код-агент (Claude Code, Cursor,
        VS Code) может обращаться к движку SpeakAgent — распознавать файлы, делать
        диаризацию, протоколы и смотреть историю. Работает офлайн, только на этом компьютере.
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
              {running ? "Сервер запущен" : "Сервер остановлен"}
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
          {running ? "Остановить" : "Запустить"}
        </button>
      </div>

      {err && <div className="mt-3 text-xs text-red-400">{err}</div>}

      {/* Подключение */}
      <div className="mt-6">
        <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">Подключение агента</h2>
        <p className="mt-2 text-xs text-zinc-500">Быстрое добавление в популярные клиенты:</p>
        <ClientButtons url={url} hasToken={status?.hasToken ?? false} />
        <p className="mt-4 text-xs text-zinc-500">
          Claude Code — выполните в терминале:
        </p>
        <CopyBlock text={`claude mcp add --transport http speakagent ${url}`} />
        <p className="mt-3 text-xs text-zinc-500">Cursor / VS Code / Claude Desktop — добавьте в конфиг MCP:</p>
        <CopyBlock
          text={JSON.stringify(
            {
              mcpServers: {
                speakagent: {
                  url,
                  ...(status?.hasToken ? { headers: { Authorization: "Bearer <ваш токен>" } } : {}),
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
        <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">Инструменты</h2>
        <div className="glass mt-3 grid grid-cols-1 gap-1.5 rounded-xl border border-white/5 p-4 sm:grid-cols-2">
          {(status?.tools ?? []).map((t) => (
            <div key={t} className="flex items-start gap-2 text-sm">
              <Wrench size={14} className="mt-0.5 shrink-0 text-amber-500/80" />
              <div>
                <span className="font-mono text-zinc-200">{t}</span>
                {TOOL_HINTS[t] && <span className="ml-1 text-xs text-zinc-500">— {TOOL_HINTS[t]}</span>}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Настройки */}
      <div className="glass mt-6 space-y-3 rounded-xl border border-white/5 p-4">
        <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">Настройки</h2>
        <label className="block">
          <span className="mb-1 block text-xs text-zinc-400">Порт</span>
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
            <ShieldCheck size={13} /> Токен (необязательно)
          </span>
          <input
            type="text"
            value={token}
            placeholder="пусто — без авторизации (только localhost)"
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
          Запускать сервер при старте приложения
        </label>
        <div className="pt-1">
          <button
            onClick={saveSettings}
            disabled={busy}
            className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400 disabled:opacity-50"
          >
            {saved ? <Check size={15} /> : null} {saved ? "Сохранено" : "Сохранить и применить"}
          </button>
        </div>
      </div>
    </div>
  );
}

function ClientButtons({ url }: { url: string; hasToken: boolean }) {
  const [copied, setCopied] = useState<string | null>(null);
  const copy = (id: string, text: string) =>
    navigator.clipboard.writeText(text).then(() => {
      setCopied(id);
      setTimeout(() => setCopied((c) => (c === id ? null : c)), 1500);
    });

  const cursorLink = `cursor://anysphere.cursor-deeplink/mcp/install?name=speakagent&config=${btoa(
    JSON.stringify({ url }),
  )}`;
  const vscodeLink = `vscode:mcp/install?${encodeURIComponent(
    JSON.stringify({ name: "speakagent", url }),
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
        title="Копировать"
        className="absolute right-2 top-2 rounded-md p-1.5 text-zinc-500 transition hover:bg-white/10 hover:text-zinc-200"
      >
        {copied ? <Check size={14} className="text-emerald-400" /> : <Copy size={14} />}
      </button>
    </div>
  );
}
