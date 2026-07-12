#!/usr/bin/env python3
"""
Фаза 0 — спайк валидации ядра SpeakAgent Desktop.

Прогоняет реальный аудио/видео файл через локальный ONNX-движок (sherpa-onnx) и
замеряет то, что решает продуктовый вопрос «тяжело ли / потянет ли слабый ноут»:
  • время распознавания и RTF (сколько считаем на 1 час аудио)
  • пиковую RAM
  • качество диаризации (кто-когда-говорил)

Ничего облачного, ничего от Rust. Декод аудио — через PyAV (ffmpeg внутри).

Примеры:
    python run_spike.py --audio "C:\\rec.mp3" --engine gigaam --diarize
    python run_spike.py --audio rec.wav --engine whisper-small --language ru
    python run_spike.py --audio meeting.mp4 --engine gigaam --diarize --num-speakers 3
"""
import argparse
import json
import sys
import threading
import time
from pathlib import Path

import numpy as np
import psutil

# Windows-консоль часто в cp1251/cp866 — форсим UTF-8, иначе кириллица/символы роняют вывод.
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8", errors="replace")
    except Exception:  # noqa: BLE001
        pass

SR = 16000  # целевая частота для всех моделей


# ────────────────────────── аудио ──────────────────────────
def load_audio_16k_mono(path: str, start_sec: float = 0.0, max_sec: float = 0.0) -> np.ndarray:
    """Декод любого формата → float32 mono 16 kHz в [-1, 1] через PyAV.
    start_sec/max_sec берут только фрагмент (не декодируя весь файл целиком)."""
    import av

    container = av.open(path)
    stream = container.streams.audio[0]
    if start_sec > 0:
        container.seek(int(start_sec * 1_000_000))  # микросекунды (AV_TIME_BASE)
    resampler = av.audio.resampler.AudioResampler(format="s16", layout="mono", rate=SR)
    chunks: list[np.ndarray] = []
    collected = 0
    limit = int(max_sec * SR) if max_sec > 0 else 0

    def emit(frames):
        # разные версии PyAV возвращают frame | list[frame] | None
        nonlocal collected
        if frames is None:
            return
        if not isinstance(frames, list):
            frames = [frames]
        for fr in frames:
            arr = fr.to_ndarray().reshape(-1)
            chunks.append(arr)
            collected += arr.shape[0]

    done = False
    for frame in container.decode(stream):
        emit(resampler.resample(frame))
        if limit and collected >= limit:
            done = True
            break
    if not done:
        emit(resampler.resample(None))  # flush
    container.close()

    if not chunks:
        raise RuntimeError("не удалось декодировать аудио (пустой поток)")
    pcm = np.concatenate(chunks).astype(np.float32) / 32768.0
    return pcm[:limit] if limit else pcm


# ────────────────────────── поиск файлов моделей ──────────────────────────
def find(models_dir: Path, *patterns: str) -> Path:
    """Первый файл, совпавший с любым glob-паттерном (рекурсивно)."""
    for pat in patterns:
        hits = sorted(models_dir.rglob(pat))
        if hits:
            return hits[0]
    raise FileNotFoundError(
        f"не найдено ни одного из {patterns} в {models_dir} — сначала запусти download_models.py"
    )


# ────────────────────────── построение распознавателя ──────────────────────────
def add_cuda_dll_dirs() -> list[str]:
    """Windows: onnxruntime CUDA EP ищет cuDNN/cuBLAS DLL; отдаём ему pip-пакеты nvidia-*."""
    if not sys.platform.startswith("win"):
        return []
    import os
    import site
    added: list[str] = []
    roots = list(site.getsitepackages())
    try:
        roots.append(site.getusersitepackages())
    except Exception:  # noqa: BLE001
        pass
    for sp in roots:
        nvidia = Path(sp) / "nvidia"
        if nvidia.is_dir():
            for binp in nvidia.glob("*/bin"):
                try:
                    os.add_dll_directory(str(binp))
                    os.environ["PATH"] = str(binp) + os.pathsep + os.environ.get("PATH", "")
                    added.append(binp.parent.name)
                except Exception:  # noqa: BLE001
                    pass
    return added


def build_recognizer(engine: str, models_dir: Path, threads: int, language: str, provider: str = "cpu"):
    import sherpa_onnx

    if engine == "gigaam":
        # GigaAM = NeMo CTC
        model = find(models_dir, "*giga-am*/model.int8.onnx", "*giga-am*/model.onnx")
        tokens = find(models_dir, "*giga-am*/tokens.txt")
        return sherpa_onnx.OfflineRecognizer.from_nemo_ctc(
            model=str(model), tokens=str(tokens), provider=provider,
            num_threads=threads, decoding_method="greedy_search", debug=False,
        )

    if engine == "whisper-small":
        enc = find(models_dir, "*whisper-small*/*encoder*.onnx")
        dec = find(models_dir, "*whisper-small*/*decoder*.onnx")
        tokens = find(models_dir, "*whisper-small*/*tokens.txt")
        return sherpa_onnx.OfflineRecognizer.from_whisper(
            encoder=str(enc), decoder=str(dec), tokens=str(tokens), provider=provider,
            num_threads=threads, language=language, task="transcribe",
        )

    if engine == "parakeet":
        # NeMo transducer (TDT)
        enc = find(models_dir, "*parakeet*/*encoder*.onnx")
        dec = find(models_dir, "*parakeet*/*decoder*.onnx")
        joiner = find(models_dir, "*parakeet*/*joiner*.onnx")
        tokens = find(models_dir, "*parakeet*/tokens.txt")
        return sherpa_onnx.OfflineRecognizer.from_transducer(
            encoder=str(enc), decoder=str(dec), joiner=str(joiner), tokens=str(tokens),
            provider=provider, num_threads=threads, model_type="nemo_transducer",
            decoding_method="greedy_search",
        )

    raise ValueError(f"неизвестный движок: {engine}")


def build_vad(models_dir: Path, max_seg: float = 20.0):
    import sherpa_onnx
    vad_model = find(models_dir, "silero_vad.onnx")
    cfg = sherpa_onnx.VadModelConfig()
    cfg.silero_vad.model = str(vad_model)
    cfg.silero_vad.threshold = 0.5
    cfg.silero_vad.min_silence_duration = 0.25
    cfg.silero_vad.min_speech_duration = 0.25
    cfg.silero_vad.max_speech_duration = max_seg
    cfg.sample_rate = SR
    if not cfg.validate():
        raise RuntimeError("VAD config validate() failed")
    return sherpa_onnx.VoiceActivityDetector(cfg, buffer_size_in_seconds=100)


def transcribe_vad(recognizer, pcm: np.ndarray, vad) -> dict:
    """Продакшн-подход: VAD режет аудио на речевые сегменты (без обрезки слов, без тишины)."""
    window = 512  # окно Silero для 16 кГц
    texts: list[str] = []
    speech = 0.0
    n_seg = 0

    def drain():
        nonlocal speech, n_seg
        while not vad.empty():
            s = vad.front
            samples = s.samples
            speech += len(samples) / SR
            n_seg += 1
            stream = recognizer.create_stream()
            stream.accept_waveform(SR, samples)
            recognizer.decode_stream(stream)
            t = (stream.result.text or "").strip()
            if t:
                texts.append(t)
            vad.pop()

    k = 0
    while k < len(pcm):
        vad.accept_waveform(pcm[k:k + window])
        k += window
        drain()
    vad.flush()
    drain()
    return {"text": " ".join(texts), "chunks": n_seg, "speech_sec": round(speech, 1)}


def transcribe(recognizer, pcm: np.ndarray, chunk_sec: float = 20.0) -> dict:
    """Offline-CTC/трансдьюсеры не берут длинное аудио за один проход — режем на окна.
    В проде окна нарезаются по VAD (Silero); здесь для замера скорости — фиксированные окна."""
    win = int(chunk_sec * SR)
    texts: list[str] = []
    n_chunks = 0
    for i in range(0, len(pcm), win):
        seg = pcm[i:i + win]
        if len(seg) < int(0.2 * SR):  # огрызок < 0.2с пропускаем
            continue
        stream = recognizer.create_stream()
        stream.accept_waveform(SR, seg)
        recognizer.decode_stream(stream)
        t = (stream.result.text or "").strip()
        if t:
            texts.append(t)
        n_chunks += 1
    return {"text": " ".join(texts), "chunks": n_chunks}


# ────────────────────────── диаризация ──────────────────────────
def build_diarizer(models_dir: Path, num_speakers: int, threads: int, cluster_threshold: float = 0.5,
                   provider: str = "cpu"):
    import sherpa_onnx

    seg = find(models_dir, "*pyannote-segmentation*/model.onnx", "*segmentation*.onnx")
    emb = find(models_dir, "*campplus*.onnx", "*cam++*.onnx", "*eres2net*.onnx", "*wespeaker*.onnx")
    config = sherpa_onnx.OfflineSpeakerDiarizationConfig(
        segmentation=sherpa_onnx.OfflineSpeakerSegmentationModelConfig(
            pyannote=sherpa_onnx.OfflineSpeakerSegmentationPyannoteModelConfig(model=str(seg)),
            num_threads=threads, provider=provider,
        ),
        embedding=sherpa_onnx.SpeakerEmbeddingExtractorConfig(model=str(emb), num_threads=threads, provider=provider),
        clustering=sherpa_onnx.FastClusteringConfig(
            num_clusters=num_speakers if num_speakers > 0 else -1,
            threshold=cluster_threshold,  # выше = меньше спикеров (борьба с пере-кластеризацией)
        ),
        min_duration_on=0.3,
        min_duration_off=0.5,
    )
    if not config.validate():
        raise RuntimeError("конфиг диаризации не прошёл validate() — проверь пути к моделям")
    return sherpa_onnx.OfflineSpeakerDiarization(config)


def diarize(diarizer, pcm: np.ndarray) -> list[dict]:
    result = diarizer.process(pcm).sort_by_start_time()
    return [{"start": round(s.start, 2), "end": round(s.end, 2), "speaker": s.speaker} for s in result]


def merge_adjacent(segs: list[dict], gap: float = 1.0) -> list[dict]:
    """Склейка соседних реплик одного спикера (порт облачного алгоритма диаризации)."""
    out: list[dict] = []
    for s in segs:
        if out and out[-1]["speaker"] == s["speaker"] and s["start"] - out[-1]["end"] <= gap:
            out[-1]["end"] = s["end"]
        else:
            out.append(dict(s))
    return out


def merged_transcript(recognizer, pcm: np.ndarray, segs: list[dict], chunk_sec: float = 20.0) -> list[dict]:
    """diarize-first → транскрипция по каждой реплике → реальный продуктовый вывод."""
    replicas: list[dict] = []
    for s in merge_adjacent(segs):
        seg = pcm[int(s["start"] * SR):int(s["end"] * SR)]
        if len(seg) < int(0.2 * SR):
            continue
        text = transcribe(recognizer, seg, chunk_sec)["text"].strip()
        if text:
            replicas.append({"speaker": s["speaker"], "start": s["start"], "end": s["end"], "text": text})
    return replicas


def fmt_ts(sec: float) -> str:
    h, rem = divmod(int(sec), 3600)
    m, s = divmod(rem, 60)
    return f"{h:d}:{m:02d}:{s:02d}"


# ────────────────────────── замер RAM ──────────────────────────
class RamSampler(threading.Thread):
    def __init__(self, interval=0.25):
        super().__init__(daemon=True)
        self.proc = psutil.Process()
        self.interval = interval
        self.peak = 0
        self._stop_flag = threading.Event()  # НЕ _stop: перекрыло бы Thread._stop

    def run(self):
        while not self._stop_flag.is_set():
            self.peak = max(self.peak, self.proc.memory_info().rss)
            time.sleep(self.interval)

    def stop(self) -> float:
        self._stop_flag.set()
        self.join(timeout=1)
        return self.peak / (1024 ** 2)  # МБ


# ────────────────────────── main ──────────────────────────
def main():
    ap = argparse.ArgumentParser(description="Спайк валидации ядра (Фаза 0)")
    ap.add_argument("--audio", required=True, help="путь к аудио/видео файлу")
    ap.add_argument("--engine", default="gigaam", choices=["gigaam", "whisper-small", "parakeet"])
    ap.add_argument("--diarize", action="store_true", help="дополнительно посчитать диаризацию")
    ap.add_argument("--num-speakers", type=int, default=0, help="0 = авто")
    ap.add_argument("--cluster-threshold", type=float, default=0.5, help="порог кластеризации диаризации (выше = меньше спикеров)")
    ap.add_argument("--start-sec", type=float, default=0.0, help="начать с этой секунды")
    ap.add_argument("--max-sec", type=float, default=0.0, help="взять только N секунд (0 = весь файл)")
    ap.add_argument("--chunk-sec", type=float, default=20.0, help="окно нарезки для ASR (сек)")
    ap.add_argument("--merge", action="store_true", help="собрать продуктовый вывод Speaker N [время]: текст")
    ap.add_argument("--vad", action="store_true", help="нарезка ASR по VAD (Silero) вместо фикс-окон")
    ap.add_argument("--language", default="ru", help="язык (для whisper)")
    ap.add_argument("--threads", type=int, default=max(1, (psutil.cpu_count(logical=False) or 2)))
    ap.add_argument("--provider", default="cpu", choices=["cpu", "cuda", "directml"], help="бэкенд ONNX Runtime")
    ap.add_argument("--models-dir", default=str(Path(__file__).parent / "models"))
    ap.add_argument("--json", action="store_true", help="вывести машинный JSON в конце")
    args = ap.parse_args()

    models_dir = Path(args.models_dir)
    print(f"→ файл:     {args.audio}")
    print(f"→ движок:   {args.engine}  (потоков: {args.threads})")
    print(f"→ модели:   {models_dir}\n")

    if args.max_sec:
        print(f"→ фрагмент: с {args.start_sec:.0f}с, длиной {args.max_sec:.0f}с")
    print("[1/4] декодирование аудио…")
    t0 = time.perf_counter()
    pcm = load_audio_16k_mono(args.audio, args.start_sec, args.max_sec)
    audio_sec = len(pcm) / SR
    print(f"      длительность: {fmt_ts(audio_sec)} ({audio_sec:.1f} c), декод за {time.perf_counter()-t0:.1f} c")

    ram = RamSampler()
    ram.start()

    if args.provider == "cuda":
        added = add_cuda_dll_dirs()
        print(f"→ провайдер: CUDA (подключены nvidia DLL: {', '.join(added) or 'нет — cuDNN может не найтись'})")

    print("[2/4] загрузка модели…")
    t0 = time.perf_counter()
    recognizer = build_recognizer(args.engine, models_dir, args.threads, args.language, args.provider)
    print(f"      модель загружена за {time.perf_counter()-t0:.1f} c")

    print(f"[3/4] транскрипция{' (VAD-нарезка)' if args.vad else ''}…")
    t0 = time.perf_counter()
    if args.vad:
        vad = build_vad(models_dir, args.chunk_sec)
        tr = transcribe_vad(recognizer, pcm, vad)
    else:
        tr = transcribe(recognizer, pcm, args.chunk_sec)
    asr_sec = time.perf_counter() - t0
    asr_rtf = asr_sec / audio_sec if audio_sec else 0
    seg_label = f"VAD-сегментов: {tr.get('chunks', 0)} (речь {tr.get('speech_sec', 0)}с)" if args.vad else f"окон: {tr.get('chunks', 0)}"
    print(f"      готово за {asr_sec:.1f} c  |  {seg_label}  |  RTF={asr_rtf:.3f}  |  скорость ×{1/asr_rtf:.1f} realtime")

    diar_segments = []
    diar_sec = 0.0
    if args.diarize:
        print("[4/4] диаризация…")
        t0 = time.perf_counter()
        diarizer = build_diarizer(models_dir, args.num_speakers, args.threads, args.cluster_threshold, args.provider)
        diar_segments = diarize(diarizer, pcm)
        diar_sec = time.perf_counter() - t0
        n_spk = len({s["speaker"] for s in diar_segments})
        print(f"      готово за {diar_sec:.1f} c  |  RTF={diar_sec/audio_sec:.3f}  |  спикеров: {n_spk}")
    else:
        print("[4/4] диаризация — пропущена (нет --diarize)")

    replicas = []
    if args.merge and diar_segments:
        print("[5/5] сборка реплик по спикерам…")
        t0 = time.perf_counter()
        replicas = merged_transcript(recognizer, pcm, diar_segments, args.chunk_sec)
        print(f"      реплик: {len(replicas)}  за {time.perf_counter()-t0:.1f} c")

    peak_mb = ram.stop()

    # ── вывод ──
    print("\n" + "=" * 64)
    print("РЕЗУЛЬТАТ")
    print("=" * 64)
    print(f"аудио:            {fmt_ts(audio_sec)}")
    print(f"ASR:              {asr_sec:.1f} c   (RTF {asr_rtf:.3f}, ×{1/asr_rtf:.1f} realtime)")
    if args.diarize:
        total = asr_sec + diar_sec
        print(f"диаризация:       {diar_sec:.1f} c   (RTF {diar_sec/audio_sec:.3f})")
        print(f"ИТОГО обработка:  {total:.1f} c   → на 1 час аудио ≈ {total/audio_sec*3600/60:.1f} мин")
    else:
        print(f"на 1 час аудио ≈  {asr_sec/audio_sec*3600/60:.1f} мин (только ASR)")
    print(f"пиковая RAM:      {peak_mb:.0f} МБ")
    print("-" * 64)

    print("\nТРАНСКРИПТ (первые 600 симв.):")
    print(tr["text"][:600] + ("…" if len(tr["text"]) > 600 else ""))

    if diar_segments and not replicas:
        print("\nДИАРИЗАЦИЯ (первые 12 сегментов):")
        for s in diar_segments[:12]:
            print(f"  Speaker{s['speaker']} [{fmt_ts(s['start'])}–{fmt_ts(s['end'])}]")

    if replicas:
        print("\nПРОДУКТОВЫЙ ВЫВОД — Speaker N [время]: текст (первые 10 реплик):")
        for r in replicas[:10]:
            print(f"  Speaker{r['speaker']} [{fmt_ts(r['start'])}]: {r['text'][:160]}")

    if args.json:
        print("\n--- JSON ---")
        print(json.dumps({
            "engine": args.engine,
            "audio_sec": round(audio_sec, 1),
            "asr_sec": round(asr_sec, 1),
            "asr_rtf": round(asr_rtf, 3),
            "diar_sec": round(diar_sec, 1),
            "peak_ram_mb": round(peak_mb),
            "num_speakers": len({s["speaker"] for s in diar_segments}) if diar_segments else None,
            "text_len": len(tr["text"]),
        }, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except Exception as e:  # noqa: BLE001
        print(f"\n✗ ошибка: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)
