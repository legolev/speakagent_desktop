import { useEffect, useState } from "react";
import { Sparkles } from "lucide-react";
import { beautifyConfig, setBeautifyConfig, type BeautifyConfig } from "../lib/api";
import { useT } from "../i18n";

/** Тумблеры украшателя в Настройках: мастер-переключатель + авто-запуск (оба выкл по умолчанию). */
export default function BeautifySettings() {
  const t = useT();
  const [cfg, setCfg] = useState<BeautifyConfig>({ enabled: false, auto: false });

  useEffect(() => {
    beautifyConfig()
      .then(setCfg)
      .catch(() => {});
  }, []);

  const update = (patch: Partial<BeautifyConfig>) => {
    const next = { ...cfg, ...patch };
    if (!next.enabled) next.auto = false; // авто без украшателя бессмысленно
    setCfg(next);
    setBeautifyConfig(next).catch(() => {});
  };

  return (
    <div className="glass mt-3 rounded-xl border border-white/5 p-4">
      <label className="flex cursor-pointer items-start gap-3">
        <input
          type="checkbox"
          checked={cfg.enabled}
          onChange={(e) => update({ enabled: e.target.checked })}
          className="mt-0.5 h-4 w-4 accent-amber-500"
        />
        <span className="min-w-0">
          <span className="flex items-center gap-1.5 text-sm font-medium text-zinc-200">
            <Sparkles size={14} className="text-amber-500" /> {t.beautify.enableLabel}
          </span>
          <span className="mt-0.5 block text-xs text-zinc-500">{t.beautify.enableHint}</span>
        </span>
      </label>
      {cfg.enabled && (
        <label className="mt-3 flex cursor-pointer items-start gap-3 border-t border-white/5 pt-3">
          <input
            type="checkbox"
            checked={cfg.auto}
            onChange={(e) => update({ auto: e.target.checked })}
            className="mt-0.5 h-4 w-4 accent-amber-500"
          />
          <span className="min-w-0">
            <span className="block text-sm text-zinc-200">{t.beautify.autoLabel}</span>
            <span className="mt-0.5 block text-xs text-zinc-500">{t.beautify.autoHint}</span>
          </span>
        </label>
      )}
    </div>
  );
}
