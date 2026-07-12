// Пре-билд: скачивает мелкие модели (~36 МБ), которые вшиваются в установщик
// (tauri.conf.json → bundle.resources). Файлы гитигнорены; скрипт идемпотентен.
// Запуск: node scripts/fetch-models.mjs  (автоматически из beforeBuildCommand)
import { existsSync, mkdirSync, createWriteStream } from "node:fs";
import { rename, rm } from "node:fs/promises";
import { pipeline } from "node:stream/promises";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(fileURLToPath(import.meta.url));
const DEST = path.join(root, "..", "src-tauri", "resources", "models");

const FILES = [
  {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx",
    check: "silero_vad.onnx",
  },
  {
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh_en_16k-common_advanced.onnx",
    check: "3dspeaker_speech_campplus_sv_zh_en_16k-common_advanced.onnx",
  },
  {
    // tar.bz2 c каталогом sherpa-onnx-pyannote-segmentation-3-0/ — распаковываем системным tar
    url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2",
    check: path.join("sherpa-onnx-pyannote-segmentation-3-0", "model.onnx"),
    extract: true,
  },
];

async function download(url, dest) {
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
  await pipeline(res.body, createWriteStream(dest));
}

mkdirSync(DEST, { recursive: true });
for (const f of FILES) {
  const checkPath = path.join(DEST, f.check);
  if (existsSync(checkPath)) {
    console.log(`✓ уже есть: ${f.check}`);
    continue;
  }
  const name = new URL(f.url).pathname.split("/").pop();
  console.log(`скачиваю ${name}…`);
  const tmp = path.join(DEST, `.${name}.part`);
  await download(f.url, tmp);
  if (f.extract) {
    // Windows 10+/macOS: системный tar (bsdtar) понимает .tar.bz2
    execFileSync("tar", ["-xjf", tmp, "-C", DEST]);
    await rm(tmp);
  } else {
    await rename(tmp, path.join(DEST, name));
  }
  if (!existsSync(checkPath)) throw new Error(`после загрузки нет ${f.check}`);
  console.log(`✓ ${f.check}`);
}
console.log("модели для установщика готовы:", DEST);
