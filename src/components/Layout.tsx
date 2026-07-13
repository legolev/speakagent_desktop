import { NavLink, Outlet } from "react-router-dom";
import { Home, FileAudio, Mic, Server, Settings, Info } from "lucide-react";
import DragDropOverlay from "./DragDropOverlay";
import StatusBar from "./StatusBar";
import { useT } from "../i18n";

export default function Layout() {
  const t = useT();
  const NAV = [
    { to: "/", label: t.nav.home, icon: Home, end: true },
    // «Расшифровка» теперь объединена с историей: сверху очередь новых записей, ниже — все прошлые.
    { to: "/transcribe", label: t.nav.transcribe, icon: FileAudio, end: false },
    { to: "/dictation", label: t.nav.dictation, icon: Mic, end: false },
    { to: "/mcp", label: t.nav.mcp, icon: Server, end: false },
    { to: "/settings", label: t.nav.settings, icon: Settings, end: false },
    { to: "/about", label: t.nav.about, icon: Info, end: false },
  ];
  return (
    <div className="flex h-screen w-screen flex-col text-zinc-100">
      <DragDropOverlay />
      <div className="flex min-h-0 flex-1">
        <aside className="glass flex w-56 flex-col border-r border-white/5 p-3">
        <div className="mb-6 px-2 pt-3">
          <div className="text-lg font-semibold tracking-tight">
            Speak<span className="text-amber-500">Agent</span>
          </div>
          <div className="text-xs text-zinc-500">{t.nav.brandSubtitle}</div>
        </div>

        <nav className="flex flex-col gap-1">
          {NAV.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                `flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition ${
                  isActive
                    ? "bg-amber-500/15 text-amber-400"
                    : "text-zinc-400 hover:bg-white/5 hover:text-zinc-100"
                }`
              }
            >
              <Icon size={18} strokeWidth={1.75} />
              {label}
            </NavLink>
          ))}
        </nav>

        <div className="mt-auto px-3 pb-2 text-[11px] text-zinc-600">{t.nav.worksOffline}</div>
      </aside>

        <main className="flex-1 overflow-y-auto bg-zinc-950/50">
          <Outlet />
        </main>
      </div>
      <StatusBar />
    </div>
  );
}
