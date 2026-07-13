// Селектор модели «Итогов встречи» (локальный LLM). Зеркалит ModelSelector:
// клик по не скачанной — выбрать и докачать (движок+модель через ensure_llm).
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Channel } from "@tauri-apps/api/core";
import { Check, Download } from "lucide-react";
import {
  listModels,
  activeLlmModel,
  setActiveLlmModel,
  ensureLlm,
  type ModelInfo,
  type DlProgress,
} from "../lib/api";
import { useT } from "../i18n";

export default function LlmModelSelector() {
  const t = useT();
  const { data: models, refetch } = useQuery({ queryKey: ["models"], queryFn: listModels });
  const { data: active, refetch: refetchActive } = useQuery({
    queryKey: ["activeLlmModel"],
    queryFn: activeLlmModel,
  });
  const [progress, setProgress] = useState<Record<string, number>>({});
  const [error, setError] = useState<Record<string, string>>({});

  const llm = (models ?? []).filter((m) => m.kind === "llm");

  async function choose(m: ModelInfo) {
    setError((e) => ({ ...e, [m.id]: "" }));
    try {
      await setActiveLlmModel(m.id);
      await refetchActive();
      if (!m.installed) {
        setProgress((p) => ({ ...p, [m.id]: 0 }));
        const ch = new Channel<DlProgress>();
        ch.onmessage = (d) =>
          setProgress((p) => ({ ...p, [m.id]: d.total > 0 ? d.done / d.total : 0 }));
        await ensureLlm(ch); // докачает движок + выбранную модель
        await refetch();
      }
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
      {llm.map((m) => {
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
              <span className="block text-sm text-zinc-200">{t.models[m.id]?.name ?? m.name}</span>
              <span className="block text-xs text-zinc-500">
                {t.common.mb(m.sizeMb)}
                {!m.installed && ` · ${t.common.notDownloaded}`}
              </span>
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
