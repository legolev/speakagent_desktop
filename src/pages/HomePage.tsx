import { Link } from "react-router-dom";
import { WifiOff, Users, Lock, ArrowRight } from "lucide-react";
import type { ComponentType } from "react";

export default function HomePage() {
  return (
    <div className="mx-auto max-w-3xl p-8">
      <h1 className="text-3xl font-semibold tracking-tight">
        Расшифровка речи — прямо на вашем компьютере
      </h1>
      <p className="mt-3 max-w-xl text-zinc-400">
        Превратите любую запись — интервью, встречу, лекцию, голосовое — в текст.
        Быстро, аккуратно и полностью офлайн: файлы никуда не загружаются.
      </p>

      <Link
        to="/transcribe"
        className="mt-7 inline-flex items-center gap-2 rounded-lg bg-amber-500 px-5 py-3 font-medium text-zinc-950 transition hover:bg-amber-400"
      >
        Расшифровать запись <ArrowRight size={18} />
      </Link>

      <div className="mt-10 grid grid-cols-1 gap-4 sm:grid-cols-3">
        <Card
          icon={WifiOff}
          title="Работает без интернета"
          text="Всё считается на вашем устройстве. Никаких загрузок и подписок за минуты."
        />
        <Card
          icon={Users}
          title="Различает говорящих"
          text="Понимает, кто и когда говорил, — удобно для встреч и интервью."
        />
        <Card
          icon={Lock}
          title="Полная приватность"
          text="Записи не покидают компьютер. Ваши разговоры остаются вашими."
        />
      </div>
    </div>
  );
}

function Card({
  icon: Icon,
  title,
  text,
}: {
  icon: ComponentType<{ size?: number; className?: string }>;
  title: string;
  text: string;
}) {
  return (
    <div className="glass rounded-xl border border-white/5 p-5">
      <Icon className="text-amber-500" size={22} />
      <div className="mt-3 font-medium">{title}</div>
      <div className="mt-1 text-sm text-zinc-400">{text}</div>
    </div>
  );
}
