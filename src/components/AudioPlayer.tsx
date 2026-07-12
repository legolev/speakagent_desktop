import { useEffect, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";

const VIDEO = /\.(mp4|mov|mkv|webm|avi|m4v|ts)$/i;

interface Props {
  path: string;
  onTime?: (sec: number) => void;
  /** Ссылку на функцию перемотки родитель использует для karaoke-клика по реплике. */
  seekRef?: React.MutableRefObject<((sec: number) => void) | null>;
}

export default function AudioPlayer({ path, onTime, seekRef }: Props) {
  const ref = useRef<HTMLMediaElement | null>(null);
  const src = convertFileSrc(path);
  const isVideo = VIDEO.test(path);

  useEffect(() => {
    if (!seekRef) return;
    seekRef.current = (sec) => {
      const el = ref.current;
      if (el) {
        el.currentTime = sec;
        void el.play();
      }
    };
    return () => {
      if (seekRef) seekRef.current = null;
    };
  }, [seekRef]);

  const onTimeUpdate = () => onTime?.(ref.current?.currentTime ?? 0);

  return isVideo ? (
    <video
      ref={ref as React.RefObject<HTMLVideoElement>}
      src={src}
      controls
      preload="metadata"
      onTimeUpdate={onTimeUpdate}
      className="max-h-64 w-full rounded-lg bg-black"
    />
  ) : (
    <audio
      ref={ref as React.RefObject<HTMLAudioElement>}
      src={src}
      controls
      preload="metadata"
      onTimeUpdate={onTimeUpdate}
      className="w-full"
    />
  );
}
