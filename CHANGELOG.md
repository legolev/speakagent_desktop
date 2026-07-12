# Changelog

All work to date. The project was built in one intensive session; entries are grouped by
theme and reference the commit.

## Phase 0 — core validated (Python spike)

Benchmarked the native ONNX engine on real audio before committing to the architecture
(Phase 0 benchmarks):
- GigaAM (RU) ×15.9, Parakeet-v3 ×24, Whisper-small ×2.7 realtime on CPU; diarization ~×14.
- RAM bounded (<1 GB at 30 min); GPU gives no gain for GigaAM/Parakeet (helps only Whisper).
- Picked the model set + confirmed "hours → minutes" on CPU.

## Phase 1 — Windows MVP

**Scaffold & engine**
- `5c245ec` Tauri 2 + React 19 + Vite 7 + Tailwind 4 scaffold; Rust core module.
- `b8161f7` sherpa-onnx ASR integrated — real offline transcription (GigaAM ×14.5 in-app);
  symphonia decode.
- `10d9ea0` speaker diarization ("who spoke when").
- `db22592` diarization rewritten to timestamp-based speaker assignment (whole-file ASR +
  per-word alignment) — big quality + speed win over per-turn.

**Formats & storage**
- `1a03142` webm/opus support via ffmpeg fallback + long-file UX.
- `cbbe8a2` persistent history via SQLite (rusqlite).
- `de202fd` export TXT / MD / PDF (PDF later redone properly).
- `fcbb298` real PDF generation in Rust with an embedded Cyrillic font (genpdf + DejaVu).

**Models**
- `086437b` model manager — catalog, download (tar.bz2/zip), path resolution (no hardcoded
  paths).
- `aec813d` default-model selector + silent background download of infra models; multi-engine
  support (CTC / transducer / Whisper).

**UX & polish**
- `7f5ba5f` human Russian copy, global job store (survives navigation), live progress, history.
- `1cec3b6` speaker rename ("Спикер 1" → name), persisted.
- `c2cb28f` first-run setup wizard; verified lean prod packaging (static-linked, models
  download on first run).
- `39c6eaf` custom app icon (amber soundwave).
- `3857468` history-menu overflow fix, drag & drop, cancel (Stop), setup wizard from Settings.
- `e674c94` inline audio/video player with **karaoke** highlight (active line + click-to-seek).
- `12fc571` block browser context menu; redesigned Transcribe + History layout (pinned
  controls/player, only the transcript scrolls).
- `ceb4c13` richer onboarding (narrative + model chooser + animation); expanded Settings
  (open data folder + "Скоро" roadmap items).

**Performance / weak PCs**
- `479b3d7` hardware detection (`system_info`), self-calibrating ETA (records real RTF per
  machine), bottom status bar (live CPU/RAM + "осталось ~N мин"), weak-config warnings, fast
  file-duration probe. Thread-sweep benchmark recorded during development.

## Quality overhaul — RU transcription + diarization

- `d9c0927` engine quality pass, driven by real-meeting testing:
  - diarization post-processing ported from the proven cloud worker (collar → max-overlap
    word assignment → A-B-A island smoothing → merge → force-single) — fixes torn replicas
    and "dozens of speakers"; multilingual CAM++ embedder replaces the zh-cn one;
    optional exact speaker count from the UI;
  - ASR chunking by Silero VAD (cuts land in silence — no more seam garbage), blind-window
    fallback; `Word` carries start+end;
  - **default RU model → GigaAM v3 CTC-punct**: punctuation + case come from the model
    itself while CTC keeps word timestamps for fine diarization;
  - RUPunct-small via pure-Rust `tract` kept as a dormant fallback for non-punct RU models
    (no second onnxruntime, single-exe preserved).

## Phase 3 — «Итоги встречи» (local LLM)

Fully offline protocols / summaries / to-do lists from transcripts.

- `c4ca44e` **slice 1** — llama-server sidecar (llama.cpp b9957, pinned), downloaded at
  runtime like ffmpeg (no C++ in the build); model catalog: Qwen3-4B-Instruct-2507 Q4_K_M
  (default) + Qwen3-1.7B (weak PCs); cloud prompts ported verbatim; headless `try_llm`
  (summary 33 s, todo 20 s on a real meeting). macOS prep: per-OS asset URLs, `tool_exe()`,
  unix exec-bit, debug-only dev fallback.
- `78db140` **slice 2** — SSE streaming, cancellation, retries; map-reduce for long
  transcripts with a cached digest (41k-char meeting: protocol 774 s, then todo via digest
  in 88 s — ~9×); idle watchdog unloads the model after 5 min; `job_results` storage +
  full IPC surface; speaker renames flow into prompts («Иван», не «Speaker 2»).
- `4e54d69` **slice 3** — UI: tabs Текст · Саммари · Протокол (деловая/собеседование) ·
  Задачи with streaming markdown, interactive todo checkboxes (persisted), first-run
  «Скачать помощника» flow, export md/txt/pdf, Settings model tier selector, status-bar
  progress.
- **slice 4** — auto-title after the first generated artifact (server is warm → seconds),
  honest slow-PC hints, docs.

## Next

Phase 2 (macOS + GPU) — most groundwork done (per-OS catalog, tool_exe, exec-bit). Deferred
items (auto-update, code signing, batch processing, repo polish before open-sourcing) are
tracked separately.
