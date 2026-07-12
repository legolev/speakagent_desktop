import { useQuery } from "@tanstack/react-query";
import {
  ShieldCheck,
  Info,
  Wand2,
  FolderOpen,
  Palette,
  Languages,
  Zap,
  Cpu,
} from "lucide-react";
import { appInfo, openDataDir, systemInfo } from "../lib/api";
import ModelSelector from "../components/ModelSelector";
import LlmModelSelector from "../components/LlmModelSelector";
import { useUi } from "../store/ui";

export default function SettingsPage() {
  const { data } = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });
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
        Итоги встречи
      </h2>
      <p className="mt-1 text-sm text-zinc-400">
        Саммари, протокол и задачи по записи составляет локальный помощник — полностью
        офлайн. К выбранной модели один раз докачивается движок (~32 МБ).
      </p>
      <AccelLine />
      <LlmModelSelector />

      <h2 className="mt-8 text-sm font-medium uppercase tracking-wide text-zinc-500">
        Скоро в обновлениях
      </h2>
      <div className="mt-3 grid grid-cols-1 gap-2 sm:grid-cols-2">
        <Soon icon={Palette} title="Темы оформления" text="Светлая тема и акценты" />
        <Soon icon={Languages} title="Язык интерфейса" text="English и другие" />
      </div>

      <div className="glass mt-8 rounded-xl border border-white/5 p-5">
        <div className="flex items-center gap-2 font-medium">
          <ShieldCheck size={18} className="text-amber-500" /> Приватность
        </div>
        <p className="mt-2 text-sm leading-relaxed text-zinc-400">
          Все записи обрабатываются только на этом компьютере и никуда не отправляются.
          Интернет нужен лишь для первой загрузки моделей.
        </p>
      </div>

      <div className="glass mt-4 rounded-xl border border-white/5 p-5">
        <div className="flex items-center gap-2 font-medium">
          <Info size={18} className="text-amber-500" /> О приложении
        </div>
        <p className="mt-2 text-sm leading-relaxed text-zinc-400">
          SpeakAgent для настольных систем · версия {data?.version ?? "…"}
        </p>
      </div>
    </div>
  );
}

/** Чем считаются «Итоги» на этой машине — видеокартой или процессором. */
function AccelLine() {
  const { data: sys } = useQuery({
    queryKey: ["systemInfo"],
    queryFn: systemInfo,
    staleTime: 60_000,
  });
  if (!sys) return null;
  return sys.llmAccel === "gpu" ? (
    <p className="mt-2 inline-flex items-center gap-1.5 text-xs text-emerald-400/90">
      <Zap size={13} /> Итоги ускоряются видеокартой ({sys.gpuName})
    </p>
  ) : (
    <p className="mt-2 inline-flex items-center gap-1.5 text-xs text-zinc-500">
      <Cpu size={13} /> Итоги считаются на процессоре
      {sys.gpuName ? ` (${sys.gpuName} не подходит для ускорения)` : ""}
    </p>
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
