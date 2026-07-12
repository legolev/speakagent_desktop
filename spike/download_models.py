#!/usr/bin/env python3
"""
Загрузчик моделей для спайка (Фаза 0).

Тянет уже-ONNX-экспортированные модели из релизов sherpa-onnx (без HuggingFace-гейтинга,
лицензии MIT/Apache) и распаковывает их в ./models/.

Примеры:
    python download_models.py --engine gigaam --diarization
    python download_models.py --engine whisper-small
    python download_models.py --all

ВНИМАНИЕ: URL'ы моделей ниже — из релизов k2-fsa/sherpa-onnx. Их теги иногда меняются;
если ссылка 404 — сверься с актуальным зоопарком:
    https://k2-fsa.github.io/sherpa/onnx/pretrained_models/index.html
    https://github.com/k2-fsa/sherpa-onnx/releases
и поправь константу URL. GigaAM и модели диаризации — самые стабильные/приоритетные для RU.
"""
import argparse
import sys
import tarfile
from pathlib import Path

import requests
from tqdm import tqdm

# Windows-консоль часто в cp1251/cp866 — форсим UTF-8, иначе '→'/'✓' роняют скрипт.
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8", errors="replace")
    except Exception:  # noqa: BLE001
        pass

REL = "https://github.com/k2-fsa/sherpa-onnx/releases/download"

# engine -> список загрузок. tarball=True → .tar.bz2 (распаковать), иначе одиночный файл.
MODELS = {
    # Лучший русский, MIT, ~226 МБ int8, ~3× realtime на 2 потоках CPU
    "gigaam": [
        (f"{REL}/asr-models/sherpa-onnx-nemo-ctc-giga-am-v2-russian-2025-04-19.tar.bz2", True),
    ],
    # Универсальный многоязычный фолбэк (98+ языков)
    "whisper-small": [
        (f"{REL}/asr-models/sherpa-onnx-whisper-small.tar.bz2", True),
    ],
    # Мультиязычный (25 EU языков вкл. RU/UK). ТЕГ МОЖЕТ ОТЛИЧАТЬСЯ — сверь при 404.
    "parakeet": [
        (f"{REL}/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2", True),
    ],
}

# Модели диаризации (маленькие): сегментация pyannote + эмбеддер CAM++ (3D-Speaker)
DIARIZATION = [
    (f"{REL}/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2", True),
    (f"{REL}/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx", False),
]

# Silero VAD (~2 МБ) — нарезка длинного аудио на речевые сегменты (продакшн-подход к чанкингу)
VAD = [
    (f"{REL}/asr-models/silero_vad.onnx", False),
]


def download(url: str, dest: Path) -> Path:
    dest.parent.mkdir(parents=True, exist_ok=True)
    fname = url.split("/")[-1]
    out = dest / fname
    if out.exists() and out.stat().st_size > 0:
        print(f"  ✓ уже есть: {fname}")
        return out
    print(f"  ↓ {fname}")
    with requests.get(url, stream=True, timeout=60) as r:
        if r.status_code != 200:
            raise RuntimeError(
                f"HTTP {r.status_code} для {url}\n"
                f"    → тег релиза мог измениться; сверься с зоопарком sherpa-onnx и поправь URL."
            )
        total = int(r.headers.get("content-length", 0))
        with open(out, "wb") as f, tqdm(
            total=total, unit="B", unit_scale=True, desc="    ", leave=False
        ) as bar:
            for chunk in r.iter_content(chunk_size=1 << 20):
                f.write(chunk)
                bar.update(len(chunk))
    return out


def maybe_extract(path: Path, tarball: bool) -> None:
    if not tarball:
        return
    print(f"  ⇲ распаковка {path.name}")
    with tarfile.open(path, "r:bz2") as t:
        t.extractall(path.parent)
    path.unlink()  # удаляем архив после распаковки


def fetch(items, models_dir: Path):
    for url, tarball in items:
        p = download(url, models_dir)
        maybe_extract(p, tarball)


def main():
    ap = argparse.ArgumentParser(description="Загрузка ONNX-моделей для спайка")
    ap.add_argument("--engine", choices=list(MODELS.keys()), help="какой ASR-движок скачать")
    ap.add_argument("--diarization", action="store_true", help="скачать модели диаризации")
    ap.add_argument("--vad", action="store_true", help="скачать Silero VAD")
    ap.add_argument("--all", action="store_true", help="скачать всё")
    ap.add_argument("--models-dir", default=str(Path(__file__).parent / "models"))
    args = ap.parse_args()

    models_dir = Path(args.models_dir)
    if not (args.engine or args.diarization or args.vad or args.all):
        ap.error("укажи --engine <name>, --diarization, --vad или --all")

    print(f"→ каталог моделей: {models_dir}")
    if args.all:
        for name, items in MODELS.items():
            print(f"[{name}]")
            fetch(items, models_dir)
        print("[diarization]")
        fetch(DIARIZATION, models_dir)
        print("[vad]")
        fetch(VAD, models_dir)
    else:
        if args.engine:
            print(f"[{args.engine}]")
            fetch(MODELS[args.engine], models_dir)
        if args.diarization:
            print("[diarization]")
            fetch(DIARIZATION, models_dir)
        if args.vad:
            print("[vad]")
            fetch(VAD, models_dir)

    print("\n✓ готово. Содержимое models/:")
    for p in sorted(models_dir.glob("*")):
        print(f"   {p.name}")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:  # noqa: BLE001
        print(f"\n✗ ошибка: {e}", file=sys.stderr)
        sys.exit(1)
