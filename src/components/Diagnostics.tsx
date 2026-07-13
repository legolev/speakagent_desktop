// Диагностика и поддержка: неизменяемый ID устройства + свёрнутая служебная информация,
// которую можно скопировать или отправить готовым issue на GitHub.
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Fingerprint, Check, Copy, Github, ChevronRight } from "lucide-react";
import { deviceId, diagnostics, openUrl } from "../lib/api";
import { useT } from "../i18n";

const REPO_ISSUES = "https://github.com/legolev/speakagent_desktop/issues/new";

export default function Diagnostics() {
  const t = useT();
  const { data: id } = useQuery({ queryKey: ["deviceId"], queryFn: deviceId });
  const { data: diag } = useQuery({ queryKey: ["diagnostics"], queryFn: diagnostics });
  const [copiedId, setCopiedId] = useState(false);
  const [copiedDiag, setCopiedDiag] = useState(false);

  function copyId() {
    void navigator.clipboard.writeText(id ?? "");
    setCopiedId(true);
    setTimeout(() => setCopiedId(false), 2000);
  }
  function copyDiag() {
    void navigator.clipboard.writeText(diag ?? "");
    setCopiedDiag(true);
    setTimeout(() => setCopiedDiag(false), 2000);
  }
  function report() {
    const body = encodeURIComponent(t.diagnostics.issueBody(diag ?? ""));
    void openUrl(`${REPO_ISSUES}?body=${body}`).catch(() => {});
  }

  return (
    <>
      {/* ID устройства */}
      <div className="glass mt-3 flex items-center gap-3 rounded-xl border border-white/5 p-4">
        <Fingerprint size={18} className="shrink-0 text-amber-500" />
        <div className="min-w-0 flex-1">
          <div className="text-xs text-zinc-500">{t.diagnostics.deviceId}</div>
          <div className="select-text font-mono text-sm tracking-wider text-zinc-200">
            {id ?? "…"}
          </div>
        </div>
        <button
          onClick={copyId}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-md border border-white/10 px-3 py-1.5 text-xs text-zinc-300 transition hover:bg-white/5"
        >
          {copiedId ? <Check size={13} /> : <Copy size={13} />}
          {copiedId ? t.common.copied : t.common.copy}
        </button>
      </div>

      {/* Служебная информация — свёрнута по умолчанию */}
      <details className="group glass mt-3 rounded-xl border border-white/5 p-4">
        <summary className="flex cursor-pointer list-none items-center gap-2 text-sm text-zinc-300">
          <ChevronRight
            size={15}
            className="text-zinc-500 transition-transform group-open:rotate-90"
          />
          {t.diagnostics.serviceInfo}
          <span className="text-xs text-zinc-600">
            {t.diagnostics.forQuestionOrBug}
          </span>
        </summary>
        <pre className="mt-3 max-h-56 overflow-auto whitespace-pre-wrap rounded-lg border border-white/5 bg-black/20 p-3 font-mono text-[11px] leading-relaxed text-zinc-400">
          {diag ?? "…"}
        </pre>
        <div className="mt-3 flex flex-wrap gap-2">
          <button
            onClick={copyDiag}
            className="inline-flex items-center gap-1.5 rounded-lg border border-white/10 px-3 py-1.5 text-xs text-zinc-200 transition hover:bg-white/5"
          >
            {copiedDiag ? <Check size={13} /> : <Copy size={13} />}
            {copiedDiag ? t.common.copied : t.diagnostics.copyAll}
          </button>
          <button
            onClick={report}
            className="inline-flex items-center gap-1.5 rounded-lg bg-amber-500 px-3 py-1.5 text-xs font-medium text-zinc-950 transition hover:bg-amber-400"
          >
            <Github size={13} /> {t.diagnostics.reportOnGithub}
          </button>
        </div>
      </details>
    </>
  );
}
