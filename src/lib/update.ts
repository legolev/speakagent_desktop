// Авто-обновление через GitHub Releases (tauri-plugin-updater).
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type { Update };

/** Проверить наличие свежего релиза. Возвращает Update или null (уже актуально). */
export async function checkUpdate(): Promise<Update | null> {
  return await check();
}

/** Скачать и установить обновление (с прогрессом в %), затем перезапустить приложение. */
export async function installUpdate(
  update: Update,
  onProgress?: (pct: number) => void,
): Promise<void> {
  let total = 0;
  let done = 0;
  await update.downloadAndInstall((e) => {
    if (e.event === "Started") {
      total = e.data.contentLength ?? 0;
    } else if (e.event === "Progress") {
      done += e.data.chunkLength;
      if (total > 0) onProgress?.(Math.min(99, Math.round((done / total) * 100)));
    } else if (e.event === "Finished") {
      onProgress?.(100);
    }
  });
  await relaunch();
}
