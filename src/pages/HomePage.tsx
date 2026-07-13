import { Link } from "react-router-dom";
import { WifiOff, Users, Lock, ArrowRight } from "lucide-react";
import type { ComponentType } from "react";
import { useT } from "../i18n";

export default function HomePage() {
  const t = useT();
  return (
    <div className="mx-auto max-w-3xl p-8">
      <h1 className="text-3xl font-semibold tracking-tight">{t.home.title}</h1>
      <p className="mt-3 max-w-xl text-zinc-400">
        {t.home.subtitle1} {t.home.subtitle2}
      </p>

      <Link
        to="/transcribe"
        className="mt-7 inline-flex items-center gap-2 rounded-lg bg-amber-500 px-5 py-3 font-medium text-zinc-950 transition hover:bg-amber-400"
      >
        {t.home.cta} <ArrowRight size={18} />
      </Link>

      <div className="mt-10 grid grid-cols-1 gap-4 sm:grid-cols-3">
        <Card
          icon={WifiOff}
          title={t.home.feat1Title}
          text={t.home.feat1Text}
        />
        <Card
          icon={Users}
          title={t.home.feat2Title}
          text={t.home.feat2Text}
        />
        <Card
          icon={Lock}
          title={t.home.feat3Title}
          text={t.home.feat3Text}
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
