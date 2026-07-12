import { useState } from "react";
import { Download } from "lucide-react";
import { exportTxt, exportMd, exportPdf } from "../lib/exporters";

interface Props {
  name: string;
  text: string;
  names?: Record<number, string>;
}

export default function ExportMenu({ name, text, names }: Props) {
  const [open, setOpen] = useState(false);

  const item = (label: string, fn: () => void) => (
    <button
      onClick={() => {
        setOpen(false);
        fn();
      }}
      className="block w-full px-3 py-2 text-left text-xs text-zinc-200 transition hover:bg-white/5"
    >
      {label}
    </button>
  );

  return (
    <div className="relative inline-block">
      <button
        onClick={() => setOpen((o) => !o)}
        className="inline-flex items-center gap-1.5 rounded-md border border-white/10 px-3 py-1.5 text-xs text-zinc-300 transition hover:bg-white/5"
      >
        <Download size={13} /> Скачать
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />
          <div className="absolute right-0 z-20 mt-1 w-36 overflow-hidden rounded-lg border border-white/10 bg-zinc-900 shadow-xl">
            {item("Текст (.txt)", () => exportTxt(name, text, names))}
            {item("Markdown (.md)", () => exportMd(name, text, names))}
            {item("PDF", () => exportPdf(name, text, names))}
          </div>
        </>
      )}
    </div>
  );
}
