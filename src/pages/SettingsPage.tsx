import { ShieldCheck, Wand2, FolderOpen, Languages } from "lucide-react";
import { openDataDir } from "../lib/api";
import ModelSelector from "../components/ModelSelector";
import AiProvider from "../components/AiProvider";
import { useUi } from "../store/ui";

export default function SettingsPage() {
  const openSetup = useUi((s) => s.openSetup);

  return (
    <div className="mx-auto max-w-3xl p-8">
      <h1 className="text-2xl font-semibold tracking-tight">Настройки</h1>

      <h2 className="mt-6 text-sm font-medium uppercase tracking-wide text-zinc-500">
        Модель по умолчанию
      </h2>
      <p className="mt-1 text-sm text-zinc-400">
        Выберите язык распознавания. Не скачанную модель можно загрузить прямо здесь —
        нажмите на неё.
      </p>
      <ModelSelector />

      <div className="mt-4 flex flex-wrap gap-2">
        <button
          onClick={openSetup}
          className="inline-flex items-center gap-2 rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-300 transition hover:bg-white/5"
        >
          <Wand2 size={15} /> Мастер настройки
        </button>
        <button
          onClick={() => openDataDir().catch(() => {})}
          className="inline-flex items-center gap-2 rounded-lg border border-white/10 px-4 py-2 text-sm text-zinc-300 transition hover:bg-white/5"
        >
          <FolderOpen size={15} /> Папка с данными
        </button>
      </div>

      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        ИИ-функции
      </h2>
      <p className="mt-1 text-sm text-zinc-400">
        Саммари, протокол и задачи по записи. Можно считать локально (полностью офлайн,
        помощник докачивается один раз ~32 МБ) или через ваш облачный ИИ по токену.
      </p>
      <AiProvider />

      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        Скоро в обновлениях
      </h2>
      <div className="mt-3 grid grid-cols-1 gap-2 sm:grid-cols-2">
        <Soon icon={Languages} title="Язык интерфейса" text="English и другие" />
      </div>

      <div className="glass mt-8 rounded-xl border border-white/5 p-5">
        <div className="flex items-center gap-2 font-medium">
          <ShieldCheck size={18} className="text-amber-500" /> Приватность
        </div>
        <p className="mt-2 text-sm leading-relaxed text-zinc-400">
          Все записи обрабатываются только на этом компьютере и никуда не отправляются.
          Интернет нужен лишь для первой загрузки моделей (и для облачного ИИ, если вы его
          выбрали).
        </p>
      </div>
    </div>
  );
}

function Soon({
  icon: Icon,
  title,
  text,
}: {
  icon: React.ComponentType<{ size?: number; className?: string }>;
  title: string;
  text: string;
}) {
  return (
    <div className="flex items-start gap-3 rounded-xl border border-white/5 bg-white/[0.02] p-4 opacity-60">
      <Icon size={18} className="mt-0.5 shrink-0 text-zinc-500" />
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm text-zinc-300">{title}</span>
          <span className="rounded bg-white/10 px-1.5 py-0.5 text-[10px] text-zinc-400">скоро</span>
        </div>
        <div className="mt-0.5 text-xs text-zinc-500">{text}</div>
      </div>
    </div>
  );
}
