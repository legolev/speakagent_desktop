import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { FileAudio } from "lucide-react";
import { useUi } from "../store/ui";

const MEDIA = /\.(mp3|m4a|aac|wav|flac|ogg|opus|oga|mp4|mov|mkv|webm|avi|m4v|ts)$/i;

export default function DragDropOverlay() {
  const [active, setActive] = useState(false);
  const navigate = useNavigate();
  const setDropped = useUi((s) => s.setDropped);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") {
          setActive(true);
        } else if (p.type === "leave") {
          setActive(false);
        } else if (p.type === "drop") {
          setActive(false);
          const path = p.paths.find((x) => MEDIA.test(x)) ?? p.paths[0];
          if (path) {
            setDropped(path);
            navigate("/transcribe");
          }
        }
      })
      .then((u) => (unlisten = u));
    return () => unlisten?.();
  }, [navigate, setDropped]);

  if (!active) return null;
  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-amber-500/10 backdrop-blur-sm">
      <div className="glass flex flex-col items-center gap-3 rounded-2xl border-2 border-dashed border-amber-500/60 px-12 py-10">
        <FileAudio size={36} className="text-amber-500" />
        <div className="text-lg font-medium text-zinc-100">Отпустите файл для расшифровки</div>
      </div>
    </div>
  );
}
