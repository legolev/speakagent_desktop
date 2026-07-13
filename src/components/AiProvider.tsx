// ИИ-функции: где считать «Итоги» — на этом компьютере (локальный llama-server)
// или через облачный OpenAI-совместимый провайдер (по токену пользователя).
import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Cpu, Zap, Cloud, Server, Save, Check, Plug, Loader2, X } from "lucide-react";
import {
  llmBackend,
  setLlmBackend,
  cloudConfig,
  setCloudConfig,
  testCloud,
  systemInfo,
  type LlmBackend,
} from "../lib/api";
import LlmModelSelector from "./LlmModelSelector";
import { useT } from "../i18n";

export default function AiProvider() {
  const t = useT();
  const qc = useQueryClient();
  const { data: backend } = useQuery({ queryKey: ["llmBackend"], queryFn: llmBackend });
  const mode: LlmBackend = backend ?? "local";

  async function switchTo(m: LlmBackend) {
    await setLlmBackend(m);
    await qc.invalidateQueries({ queryKey: ["llmBackend"] });
    await qc.invalidateQueries({ queryKey: ["llmReady"] });
  }

  return (
    <div className="mt-3">
      {/* Переключатель локально / облако */}
      <div className="inline-flex rounded-lg border border-white/10 bg-white/5 p-0.5">
        {(
          [
            { id: "local", label: t.aiProvider.localLabel, icon: Server },
            { id: "cloud", label: t.aiProvider.cloudLabel, icon: Cloud },
          ] as const
        ).map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => switchTo(id)}
            className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs transition ${
              mode === id ? "bg-amber-500 text-zinc-950" : "text-zinc-300 hover:text-zinc-100"
            }`}
          >
            <Icon size={13} /> {label}
          </button>
        ))}
      </div>

      {mode === "local" ? (
        <>
          <AccelLine />
          <LlmModelSelector />
        </>
      ) : (
        <CloudForm />
      )}
    </div>
  );
}

/** Чем считаются локальные «Итоги» — видеокартой или процессором. */
function AccelLine() {
  const t = useT();
  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });
  if (!sys) return null;
  return sys.llmAccel === "gpu" ? (
    <p className="mt-3 inline-flex items-center gap-1.5 text-xs text-emerald-400/90">
      <Zap size={13} /> {t.aiProvider.gpuAccel(sys.gpuName)}
    </p>
  ) : (
    <p className="mt-3 inline-flex items-center gap-1.5 text-xs text-zinc-500">
      <Cpu size={13} /> {t.aiProvider.noGpuAccel}
      {sys.gpuName ? t.aiProvider.gpuUnfit(sys.gpuName) : ""}
    </p>
  );
}

/** Настройки облачного провайдера: адрес, модель, токен. */
function CloudForm() {
  const t = useT();
  const { data: cfg } = useQuery({ queryKey: ["cloudConfig"], queryFn: cloudConfig });
  const [url, setUrl] = useState("");
  const [model, setModel] = useState("");
  const [key, setKey] = useState("");
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState("");
  const [testing, setTesting] = useState(false);
  const [testOk, setTestOk] = useState<string | null>(null); // ответ модели при успехе
  const [testErr, setTestErr] = useState("");

  useEffect(() => {
    if (cfg) {
      setUrl(cfg.url);
      setModel(cfg.model);
      setKey(cfg.key);
    }
  }, [cfg]);

  async function save() {
    setError("");
    try {
      await setCloudConfig(url, model, key);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    }
  }

  async function test() {
    setTesting(true);
    setTestOk(null);
    setTestErr("");
    try {
      const reply = await testCloud(url, model, key);
      setTestOk(reply || t.aiProvider.testOkFallback);
    } catch (e) {
      setTestErr(String(e));
    } finally {
      setTesting(false);
    }
  }

  return (
    <div className="glass mt-4 space-y-3 rounded-xl border border-white/5 p-4">
      <p className="text-xs leading-relaxed text-zinc-500">{t.aiProvider.cloudIntro}</p>
      <Field label={t.aiProvider.apiUrl} value={url} onChange={setUrl} placeholder="https://openrouter.ai/api/v1" />
      <Field label={t.aiProvider.model} value={model} onChange={setModel} placeholder="openai/gpt-4o-mini" />
      <Field label={t.aiProvider.token} value={key} onChange={setKey} placeholder="sk-…" password />
      <div className="flex flex-wrap items-center gap-3 pt-1">
        <button
          onClick={save}
          className="inline-flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-zinc-950 transition hover:bg-amber-400"
        >
          {saved ? <Check size={15} /> : <Save size={15} />} {saved ? t.common.saved : t.common.save}
        </button>
        <button
          onClick={test}
          disabled={testing || !key.trim()}
          className="inline-flex items-center gap-2 rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-200 transition hover:bg-white/5 disabled:cursor-default disabled:opacity-40"
        >
          {testing ? <Loader2 size={15} className="animate-spin" /> : <Plug size={15} />}
          {testing ? t.aiProvider.testing : t.aiProvider.test}
        </button>
        {error && <span className="text-xs text-red-400">{error}</span>}
      </div>
      {testOk !== null && (
        <div className="inline-flex items-center gap-1.5 text-xs text-emerald-400">
          <Check size={13} /> {t.aiProvider.testSuccess(testOk)}
        </div>
      )}
      {testErr && (
        <div className="inline-flex items-start gap-1.5 text-xs text-red-400">
          <X size={13} className="mt-0.5 shrink-0" /> {testErr}
        </div>
      )}
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  placeholder,
  password,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  password?: boolean;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs text-zinc-400">{label}</span>
      <input
        type={password ? "password" : "text"}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        spellCheck={false}
        autoComplete="off"
        className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-amber-500/50"
      />
    </label>
  );
}
