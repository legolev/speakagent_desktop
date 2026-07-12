import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Channel } from "@tauri-apps/api/core";
import { Check, Download } from "lucide-react";
import {
  listModels,
  activeModel,
  setActiveModel,
  downloadModel,
  type ModelInfo,
  type DlProgress,
} from "../lib/api";

export default function ModelSelector() {
  const { data: models, refetch } = useQuery({ queryKey: ["models"], queryFn: listModels });
  const { data: active, refetch: refetchActive } = useQuery({
    queryKey: ["activeModel"],
    queryFn: activeModel,
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
              <span className="block text-sm text-zinc-200">{m.name}</span>
              <span className="block text-xs text-zinc-500">
                {m.lang} · {m.sizeMb} МБ{!m.installed && " · не скачана"}
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
