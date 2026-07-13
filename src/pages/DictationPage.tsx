import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Mic,
  Keyboard,
  Volume2,
  VolumeX,
  ClipboardPaste,
  Copy,
  Trash2,
  Search,
  Save,
  Check,
  Loader2,
  AlertCircle,
  Radio,
  ShieldCheck,
  ShieldAlert,
} from "lucide-react";
import {
  dictationConfig,
  setDictationConfig,
  listInputDevices,
  dictationStart,
  dictationStop,
  listModels,
  permissionsStatus,
  requestPermission,
  type DictationConfig,
} from "../lib/api";
import { useDictation } from "../store/dictation";
import { fmtDuration, fmtDate } from "../lib/format";
import { useT } from "../i18n";

// Browser KeyboardEvent.code → имя клавиши rdev (то, что видит бэкенд-слушатель).
const CODE_TO_RDEV: Record<string, string> = {
  ShiftLeft: "ShiftLeft",
  ShiftRight: "ShiftRight",
  ControlLeft: "ControlLeft",
  ControlRight: "ControlRight",
  AltLeft: "Alt",
  AltRight: "AltGr",
  MetaLeft: "MetaLeft",
  MetaRight: "MetaRight",
  Space: "Space",
  Enter: "Return",
  Tab: "Tab",
  Escape: "Escape",
  Backspace: "Backspace",
  CapsLock: "CapsLock",
  ArrowUp: "UpArrow",
  ArrowDown: "DownArrow",
  ArrowLeft: "LeftArrow",
  ArrowRight: "RightArrow",
};

function rdevName(e: KeyboardEvent): string | null {
  const c = e.code;
  if (CODE_TO_RDEV[c]) return CODE_TO_RDEV[c];
  if (/^Key[A-Z]$/.test(c)) return c; // KeyA
  if (/^Digit[0-9]$/.test(c)) return "Num" + c.slice(5); // Digit1 → Num1
  if (/^F([1-9]|1[0-2])$/.test(c)) return c; // F1..F12
  if (/^Numpad[0-9]$/.test(c)) return "Kp" + c.slice(6); // Numpad1 → Kp1
  return null;
}

export default function DictationPage() {
  const t = useT();
  // Человекочитаемая подпись клавиши(-ш).
  const keyLabels = t.dictation.keyLabels as Record<string, string>;
  function prettyKey(spec: string): string {
    if (!spec) return "—";
    return spec
      .split("+")
      .map((k) => keyLabels[k] ?? k.replace(/^Key/, "").replace(/^Num/, "").replace(/^Kp/, "Num "))
      .join(" + ");
  }

  const entries = useDictation((s) => s.entries);
  const recording = useDictation((s) => s.recording);
  const processing = useDictation((s) => s.processing);
  const hydrate = useDictation((s) => s.hydrate);
  const remove = useDictation((s) => s.remove);
  const clear = useDictation((s) => s.clear);
  const error = useDictation((s) => s.error);
  const clearError = useDictation((s) => s.clearError);

  const [cfg, setCfg] = useState<DictationConfig | null>(null);
  const [saved, setSaved] = useState(false);
  const [saveErr, setSaveErr] = useState("");
  const [capturing, setCapturing] = useState(false);
  const [search, setSearch] = useState("");
  const [confirmClear, setConfirmClear] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const { data: devices } = useQuery({ queryKey: ["inputDevices"], queryFn: listInputDevices });
  const { data: models } = useQuery({ queryKey: ["models"], queryFn: listModels });
  const asrModels = (models ?? []).filter((m) => m.kind === "asr" && m.installed);
  // Разрешения macOS — опрашиваем периодически, чтобы галочки обновлялись после выдачи.
  const { data: perms } = useQuery({
    queryKey: ["permissions"],
    queryFn: permissionsStatus,
    refetchInterval: 3000,
  });

  useEffect(() => {
    void hydrate();
    dictationConfig().then(setCfg).catch(() => {});
  }, [hydrate]);

  // Захват клавиши/комбинации: копим зажатые, финализируем набор при полном отпускании.
  // Так одиночный правый Shift → "ShiftRight", а Ctrl+Space → "ControlLeft+Space".
  const heldRef = useRef<Set<string>>(new Set());
  const peakRef = useRef<string[]>([]);
  useEffect(() => {
    if (!capturing) return;
    heldRef.current = new Set();
    peakRef.current = [];
    function onDown(e: KeyboardEvent) {
      e.preventDefault();
      const name = rdevName(e);
      if (!name) return;
      heldRef.current.add(name);
      if (heldRef.current.size > peakRef.current.length) {
        peakRef.current = Array.from(heldRef.current);
      }
    }
    function onUp(e: KeyboardEvent) {
      e.preventDefault();
      const name = rdevName(e);
      if (name) heldRef.current.delete(name);
      if (heldRef.current.size === 0 && peakRef.current.length > 0) {
        const spec = peakRef.current.join("+");
        setCfg((c) => (c ? { ...c, hotkey: spec } : c));
        setCapturing(false);
      }
    }
    window.addEventListener("keydown", onDown, true);
    window.addEventListener("keyup", onUp, true);
    return () => {
      window.removeEventListener("keydown", onDown, true);
      window.removeEventListener("keyup", onUp, true);
    };
  }, [capturing]);

  async function save() {
    if (!cfg) return;
    setSaveErr("");
    try {
      await setDictationConfig(cfg);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setSaveErr(String(e));
    }
  }

  function copy(id: string, text: string) {
    navigator.clipboard.writeText(text).then(() => {
      setCopiedId(id);
      setTimeout(() => setCopiedId((c) => (c === id ? null : c)), 1500);
    });
  }

  const q = search.trim().toLowerCase();
  const shown = q ? entries.filter((e) => e.text.toLowerCase().includes(q)) : entries;

  return (
    <div className="mx-auto max-w-3xl p-8">
      <div className="flex items-center gap-3">
        <Mic size={22} className="text-amber-500" />
        <h1 className="text-2xl font-semibold tracking-tight">{t.dictation.title}</h1>
      </div>
      <p className="mt-2 text-sm leading-relaxed text-zinc-400">{t.dictation.intro}</p>

      {/* Индикатор состояния + кнопка-тест */}
      <div className="glass mt-5 flex items-center justify-between gap-4 rounded-xl border border-white/5 p-4">
        <div className="flex items-center gap-3">
          {recording ? (
            <span className="inline-flex items-center gap-2 text-sm text-red-400">
              <span className="relative flex h-3 w-3">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-red-500 opacity-70" />
                <span className="relative inline-flex h-3 w-3 rounded-full bg-red-500" />
              </span>
              {t.dictation.recording}
            </span>
          ) : processing ? (
            <span className="inline-flex items-center gap-2 text-sm text-amber-400">
              <Loader2 size={14} className="animate-spin" /> {t.dictation.processing}
            </span>
          ) : (
            <span className="inline-flex items-center gap-2 text-sm text-zinc-400">
              <Radio size={14} /> {t.dictation.ready}
            </span>
          )}
        </div>
        <button
          onMouseDown={() => dictationStart().catch(() => {})}
          onMouseUp={() => dictationStop().catch(() => {})}
          onMouseLeave={() => recording && dictationStop().catch(() => {})}
          className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition select-none hover:bg-amber-400 active:scale-95"
        >
          <Mic size={15} /> {t.dictation.holdToRecord}
        </button>
      </div>

      {error && (
        <div className="mt-3 flex items-start gap-2 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-300">
          <AlertCircle size={14} className="mt-0.5 shrink-0" />
          <span className="flex-1">{error}</span>
          <button onClick={clearError} className="text-red-400 hover:text-red-200">
            ✕
          </button>
        </div>
      )}

      {/* Настройки */}
      {cfg && (
        <div className="glass mt-5 space-y-4 rounded-xl border border-white/5 p-4">
          <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">{t.dictation.settings}</h2>

          {/* Хоткей */}
          <div>
            <span className="mb-1 block text-xs text-zinc-400">{t.dictation.hotkeyLabel}</span>
            <div className="flex items-center gap-2">
              <div className="flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-zinc-100">
                {prettyKey(cfg.hotkey)}
              </div>
              <button
                onClick={() => setCapturing((v) => !v)}
                className={`inline-flex items-center gap-1.5 rounded-lg border px-3 py-2 text-sm transition ${
                  capturing
                    ? "border-amber-500/60 bg-amber-500/15 text-amber-400"
                    : "border-white/10 text-zinc-300 hover:bg-white/5"
                }`}
              >
                <Keyboard size={14} /> {capturing ? t.dictation.capturing : t.dictation.record}
              </button>
            </div>
          </div>

          {/* Режим */}
          <div>
            <span className="mb-1 block text-xs text-zinc-400">{t.dictation.mode}</span>
            <div className="inline-flex rounded-lg border border-white/10 bg-white/5 p-0.5">
              {(
                [
                  { id: "hold", label: t.dictation.modeHold },
                  { id: "toggle", label: t.dictation.modeToggle },
                ] as const
              ).map(({ id, label }) => (
                <button
                  key={id}
                  onClick={() => setCfg({ ...cfg, mode: id })}
                  className={`rounded-md px-3 py-1.5 text-xs transition ${
                    cfg.mode === id ? "bg-amber-500 text-zinc-950" : "text-zinc-300 hover:text-zinc-100"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>

          {/* Тумблеры */}
          <div className="flex flex-wrap gap-2">
            <Toggle
              on={cfg.autopaste}
              onClick={() => setCfg({ ...cfg, autopaste: !cfg.autopaste })}
              icon={ClipboardPaste}
              label={t.dictation.autopaste}
            />
            <Toggle
              on={cfg.sound}
              onClick={() => setCfg({ ...cfg, sound: !cfg.sound })}
              icon={cfg.sound ? Volume2 : VolumeX}
              label={t.dictation.sound}
            />
          </div>

          {/* Устройство ввода */}
          <label className="block">
            <span className="mb-1 block text-xs text-zinc-400">{t.dictation.mic}</span>
            <select
              value={cfg.inputDevice}
              onChange={(e) => setCfg({ ...cfg, inputDevice: e.target.value })}
              className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-zinc-100 outline-none focus:border-amber-500/50"
            >
              <option value="">{t.dictation.defaultDevice}</option>
              {(devices ?? []).map((d) => (
                <option key={d} value={d}>
                  {d}
                </option>
              ))}
            </select>
          </label>

          {/* Модель распознавания */}
          <label className="block">
            <span className="mb-1 block text-xs text-zinc-400">{t.dictation.asrModel}</span>
            <select
              value={cfg.model}
              onChange={(e) => setCfg({ ...cfg, model: e.target.value })}
              className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-zinc-100 outline-none focus:border-amber-500/50"
            >
              <option value="">{t.dictation.asActive}</option>
              {asrModels.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
          </label>

          <div className="flex flex-wrap items-center gap-3 pt-1">
            <button
              onClick={save}
              className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
            >
              {saved ? <Check size={15} /> : <Save size={15} />} {saved ? t.common.saved : t.common.save}
            </button>
            {saveErr && <span className="text-xs text-red-400">{saveErr}</span>}
          </div>

          <p className="text-xs leading-relaxed text-zinc-600">{t.dictation.macHelp}</p>
        </div>
      )}

      {/* Разрешения (только macOS) */}
      {perms?.needed && (
        <div className="glass mt-5 space-y-2 rounded-xl border border-white/5 p-4">
          <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">
            {t.dictation.permsTitle}
          </h2>
          <p className="text-xs leading-relaxed text-zinc-600">{t.dictation.permsIntro}</p>
          <PermRow
            granted={perms.inputMonitoring}
            title={t.dictation.permInputTitle}
            desc={t.dictation.permInputDesc}
            onRequest={() => requestPermission("input-monitoring")}
          />
          <PermRow
            granted={perms.accessibility}
            title={t.dictation.permAccessTitle}
            desc={t.dictation.permAccessDesc}
            onRequest={() => requestPermission("accessibility")}
          />
        </div>
      )}

      {/* История */}
      <div className="mt-8 flex items-center justify-between gap-3">
        <h2 className="text-sm font-medium uppercase tracking-wide text-zinc-500">{t.dictation.historyTitle}</h2>
        <div className="flex items-center gap-2">
          {entries.length > 0 && (
            <div className="relative">
              <Search
                size={14}
                className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-500"
              />
              <input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder={t.dictation.searchPlaceholder}
                className="w-52 rounded-lg border border-white/10 bg-white/5 py-1.5 pl-8 pr-3 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-500/50"
              />
            </div>
          )}
          {entries.length > 0 && (
            <button
              onClick={() => setConfirmClear(true)}
              className="inline-flex items-center gap-1.5 rounded-lg border border-white/10 px-3 py-1.5 text-xs text-zinc-400 transition hover:bg-white/5 hover:text-red-300"
            >
              <Trash2 size={13} /> {t.common.clear}
            </button>
          )}
        </div>
      </div>

      {entries.length === 0 ? (
        <div className="glass mt-3 flex flex-col items-center gap-2 rounded-xl border border-white/5 p-10 text-center">
          <Mic size={26} className="text-zinc-700" />
          <div className="text-sm text-zinc-400">{t.dictation.emptyHint}</div>
        </div>
      ) : shown.length === 0 ? (
        <div className="mt-3 rounded-xl border border-white/5 p-6 text-center text-sm text-zinc-500">
          {t.common.noResultsFor(search)}
        </div>
      ) : (
        <div className="mt-3 overflow-hidden rounded-xl border border-white/5">
          {shown.map((d) => (
            <div
              key={d.id}
              className="grid grid-cols-[1fr_5rem_7rem_4rem] items-center gap-3 border-b border-white/5 px-4 py-3 transition last:border-b-0 hover:bg-white/5"
            >
              <span className="truncate text-sm text-zinc-200" title={d.text}>
                {d.text}
              </span>
              <span className="text-xs text-zinc-500">{fmtDuration(d.durationSec)}</span>
              <span className="text-xs text-zinc-500">{fmtDate(d.createdAt)}</span>
              <div className="flex items-center justify-end gap-1">
                <button
                  onClick={() => copy(d.id, d.text)}
                  title={t.common.copy}
                  className="rounded-md p-1.5 text-zinc-500 transition hover:bg-white/5 hover:text-zinc-200"
                >
                  {copiedId === d.id ? (
                    <Check size={15} className="text-emerald-400" />
                  ) : (
                    <Copy size={15} />
                  )}
                </button>
                <button
                  onClick={() => remove(d.id)}
                  title={t.common.delete}
                  className="rounded-md p-1.5 text-zinc-500 transition hover:bg-white/5 hover:text-red-400"
                >
                  <Trash2 size={15} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {confirmClear && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-6">
          <div className="glass w-full max-w-md rounded-2xl border border-white/10 p-6">
            <div className="text-lg font-semibold text-zinc-100">{t.dictation.clearConfirmTitle}</div>
            <p className="mt-2 text-sm leading-relaxed text-zinc-400">{t.dictation.clearConfirmBody}</p>
            <div className="mt-5 flex justify-end gap-2">
              <button
                onClick={() => setConfirmClear(false)}
                className="rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-300 transition hover:bg-white/5"
              >
                {t.common.cancel}
              </button>
              <button
                onClick={() => {
                  clear();
                  setConfirmClear(false);
                }}
                className="inline-flex items-center gap-2 rounded-lg bg-red-500 px-4 py-2 text-sm font-medium text-white transition hover:bg-red-400"
              >
                <Trash2 size={15} /> {t.common.clear}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function PermRow({
  granted,
  title,
  desc,
  onRequest,
}: {
  granted: boolean;
  title: string;
  desc: string;
  onRequest: () => Promise<void>;
}) {
  const t = useT();
  return (
    <div className="flex items-center gap-3 rounded-lg border border-white/5 bg-white/[0.02] px-3 py-2">
      {granted ? (
        <ShieldCheck size={18} className="shrink-0 text-emerald-400" />
      ) : (
        <ShieldAlert size={18} className="shrink-0 text-amber-400" />
      )}
      <div className="min-w-0 flex-1">
        <div className="text-sm text-zinc-200">{title}</div>
        <div className="truncate text-xs text-zinc-500">{desc}</div>
      </div>
      {granted ? (
        <span className="inline-flex items-center gap-1 text-xs text-emerald-400">
          <Check size={13} /> {t.dictation.permGranted}
        </span>
      ) : (
        <button
          onClick={() => onRequest().catch(() => {})}
          className="shrink-0 rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-1.5 text-xs text-amber-300 transition hover:bg-amber-500/20"
        >
          {t.dictation.permRequest}
        </button>
      )}
    </div>
  );
}

function Toggle({
  on,
  onClick,
  icon: Icon,
  label,
}: {
  on: boolean;
  onClick: () => void;
  icon: typeof Volume2;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      className={`inline-flex items-center gap-2 rounded-lg border px-3 py-2 text-sm transition ${
        on
          ? "border-amber-500/50 bg-amber-500/15 text-amber-300"
          : "border-white/10 text-zinc-400 hover:bg-white/5"
      }`}
    >
      <Icon size={15} /> {label}
      <span
        className={`ml-1 inline-block h-2 w-2 rounded-full ${on ? "bg-amber-400" : "bg-zinc-600"}`}
      />
    </button>
  );
}
