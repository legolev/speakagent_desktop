import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Mic, Loader2 } from "lucide-react";
import { useT } from "../i18n";

/** Плавающий индикатор записи (отдельное окно `dict-overlay`). Фон прозрачный. */
export default function OverlayPage() {
  const t = useT();
  // Окно показывается только во время записи → по умолчанию считаем, что пишем.
  const [processing, setProcessing] = useState(false);

  useEffect(() => {
    const un = listen<{ recording: boolean; processing: boolean }>("dictation:state", (e) => {
      setProcessing(e.payload.processing);
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  return (
    <div className="flex h-full w-full items-center justify-center bg-transparent">
      <div className="flex items-center gap-2.5 rounded-full border border-white/10 bg-zinc-900/80 px-4 py-2 shadow-lg backdrop-blur-md">
        {processing ? (
          <>
            <Loader2 size={16} className="animate-spin text-amber-400" />
            <span className="text-sm font-medium text-zinc-200">{t.overlay.recognizing}</span>
          </>
        ) : (
          <>
            <span className="relative flex h-3 w-3">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-red-500 opacity-70" />
              <span className="relative inline-flex h-3 w-3 rounded-full bg-red-500" />
            </span>
            <Mic size={16} className="text-zinc-200" />
            <span className="text-sm font-medium text-zinc-200">{t.overlay.recording}</span>
          </>
        )}
      </div>
    </div>
  );
}
