import { ShieldCheck, Wand2, FolderOpen, Globe } from "lucide-react";
import { openDataDir } from "../lib/api";
import ModelSelector from "../components/ModelSelector";
import AiProvider from "../components/AiProvider";
import { useUi } from "../store/ui";
import { useT, useLang } from "../i18n";

export default function SettingsPage() {
  const t = useT();
  const openSetup = useUi((s) => s.openSetup);
  const lang = useLang((s) => s.lang);
  const setLang = useLang((s) => s.setLang);

  return (
    <div className="mx-auto max-w-3xl p-8">
      <h1 className="text-2xl font-semibold tracking-tight">{t.settings.title}</h1>

      <h2 className="mt-6 text-sm font-medium uppercase tracking-wide text-zinc-500">
        {t.settings.defaultModel}
      </h2>
      <p className="mt-1 text-sm text-zinc-400">{t.settings.defaultModelHint}</p>
      <ModelSelector />

      <div className="mt-4 flex flex-wrap gap-2">
        <button
          onClick={openSetup}
          className="inline-flex items-center gap-2 rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-300 transition hover:bg-white/5"
        >
          <Wand2 size={15} /> {t.settings.setupWizard}
        </button>
        <button
          onClick={() => openDataDir().catch(() => {})}
          className="inline-flex items-center gap-2 rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-300 transition hover:bg-white/5"
        >
          <FolderOpen size={15} /> {t.settings.dataFolder}
        </button>
      </div>

      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        {t.settings.aiFeatures}
      </h2>
      <p className="mt-1 text-sm text-zinc-400">{t.settings.aiFeaturesHint}</p>
      <AiProvider />

      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        {t.settings.interfaceLanguage}
      </h2>
      <p className="mt-1 text-sm text-zinc-400">{t.settings.interfaceLanguageHint}</p>
      <div className="mt-3 inline-flex items-center gap-2">
        <Globe size={16} className="text-zinc-500" />
        <div className="inline-flex overflow-hidden rounded-lg border border-white/10">
          <button
            onClick={() => setLang("ru")}
            className={`px-4 py-2 text-sm transition ${
              lang === "ru"
                ? "bg-amber-500/15 text-amber-400"
                : "text-zinc-300 hover:bg-white/5"
            }`}
          >
            Русский
          </button>
          <button
            onClick={() => setLang("en")}
            className={`border-l border-white/10 px-4 py-2 text-sm transition ${
              lang === "en"
                ? "bg-amber-500/15 text-amber-400"
                : "text-zinc-300 hover:bg-white/5"
            }`}
          >
            English
          </button>
        </div>
      </div>

      <div className="glass mt-8 rounded-xl border border-white/5 p-5">
        <div className="flex items-center gap-2 font-medium">
          <ShieldCheck size={18} className="text-amber-500" /> {t.settings.privacy}
        </div>
        <p className="mt-2 text-sm leading-relaxed text-zinc-400">
          {t.settings.privacyText}
        </p>
      </div>
    </div>
  );
}
