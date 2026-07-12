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

## Phase 2 — macOS (Apple Silicon)

Brought the app up on Apple Silicon. The whole native stack compiles and links against the
**system toolchain** (Apple clang / current macOS SDK) — the "single self-contained binary,
no side dylibs" property holds (`otool -L` shows only `/usr/lib/libc++`).

**Verified end-to-end (headless + full app):**
- decode (symphonia) → **GigaAM v3 RU** ASR with punctuation/case → **diarization** →
  replica assignment; **PDF** with the embedded Cyrillic font.
- «Итоги встречи» on **Metal** — `llama-server` sidecar; `should_offload()` auto-detects the
  Apple-Silicon accelerator (`--list-devices` → `MTL0`), generation runs on GPU.
- Full `.app` launches with native **vibrancy**, models resolve from the bundled
  `Contents/Resources/models`, hardware + GPU acceleration shown in the UI.

**Fixes found on macOS:**
- **AAC decode (cross-platform, high-impact).** symphonia's AAC decoder returns
  plausible-but-wrong samples for many `.mp4`/`.m4a` files (it doesn't error → the ffmpeg
  fallback never fired) → garbled transcription. Diagnosed on a real file: same clip, symphonia
  → gibberish, ffmpeg → clean; isolated to the AAC decoder (resampler is fine). Fix: `decode.rs`
  now routes AAC containers (`mp4/m4a/m4v/aac/mov/3gp`) through **ffmpeg** first, symphonia kept
  as fallback. Affects the most common phone/screen-recording format (iPhone memos are m4a/AAC).
- **Whisper produced empty output (cross-platform).** sherpa returns whisper text with
  `timestamps = Some(empty)`; `decode_segment` took the timestamped branch and `push_words`
  zipped tokens against zero timestamps → every word dropped. Now falls back to the segment
  text when timestamps are empty (diarization coarsens to VAD-segment granularity, as expected
  for whisper). Fixes both whisper-small and whisper-turbo.
- llama macOS `tar.gz` extraction dropped the versioned dylib **symlinks** (`libX.0.dylib →
  libX.0.0.NNNN.dylib`) → `dyld: Library not loaded @rpath/…`; symlinks are now recreated
  (`engine/models.rs`). Windows was unaffected (flat `.dll`s).
- `gpu::best_gpu()` returned `None` on macOS (no discrete adapters) → empty "()" in the Итоги
  GPU label; now reports the Metal accelerator.
- `MIN_VRAM_BYTES` gated to Windows (dead code on macOS); cross-platform slashes in the
  `try_transcribe` dev fallback.

**Models:**
- Added **Whisper large-v3-turbo** to the catalog (sherpa `whisper-turbo`, ~540 MB) — a
  noise/music-robust multilingual option (heavy/slow on CPU; for strong machines or hard audio).
- **Per-model hardware-fit badge** in the model list — green/amber/red by the machine's
  RAM + CPU vs the model's footprint and compute cost.

**UI:**
- **Player / transcript split into two panes** (matches the web cabinet): media player pinned
  in a left column, tabs + scrolling transcript on the right (`ResultView`). Karaoke
  click-to-seek preserved.
- **Итоги format switches (Текст / Саммари / Протокол / Задачи) moved** from horizontal tabs to
  a vertical button list under the player, in the left column.
- **Расшифровка and История merged** into one hub: pick or drag **multiple** files (they form a
  sequential **queue**), with the full transcript history as a table below. Removed the separate
  История nav.
- Clicking a history row opens the transcript as a **separate detail screen** with a Назад button
  (no longer an inline accordion). History table gained a **duration** column (persisted in SQLite).
- **«Расшифровать заново»** button under the player — re-runs the transcription of a saved
  recording in place (e.g. to try another model).
- Model-fit badges reworded (Хорошо подойдёт / Подойдёт / Тяжело для этого ПК) and now factor in
  the **accelerator** (Apple Silicon / discrete GPU). The bottom status bar shows the GPU too.

**AI (Итоги → «ИИ-функции»):**
- Section renamed to **ИИ-функции**. Added a **cloud AI provider** option alongside the local
  engine: OpenAI-compatible base URL (default OpenRouter), model (default `openai/gpt-4o-mini`),
  and an API token (stored locally). When cloud is selected, summaries/protocols/to-dos are
  generated via the provider (`engine/llm.rs::cloud_chat`, single call — no local map-reduce);
  `is_ready`/backend routing updated. The token is the user's own and never leaves the machine
  except to their chosen provider.
- **«Проверить связь»** button in the cloud form — runs a tiny request (`test_cloud`) and reports
  success (model reply) or a readable error (bad token / model / URL).

**Polish (feedback round):**
- The transcription now records the **real audio duration** on the backend (from the decoded
  sample count) and shows it in the history table's duration column — reliable regardless of the
  file format. The audio/video file is never copied: only its path is stored.
- Weak-hardware warning is now based on the machine's **total** RAM (`< 8 GB`), not the momentary
  free memory (which macOS routinely under-reports) — no more false "low memory" alarm.
- Reworded the local-engine acceleration line; dropped the "Темы оформления" roadmap card.
- New **«О приложении»** page (own nav entry): version, source repo (opens in the browser via
  `open_url`), license, hardware/accelerator, and a live component-status panel — plus a small
  easter egg.
- History table gained **search** (by name **and transcript content**); a recording can be
  **renamed** in-app (click the title in the detail header) and its **file path is shown and
  clickable** — reveals the original file in Finder/Explorer (`reveal_file`, never a copy — only
  the path is stored).
- Settings → **«Диагностика и поддержка»**: a stable, immutable **device ID** (hardware machine-id
  → FNV-1a fingerprint, raw UUID not exposed) and a collapsible **service-info** block — one click
  to copy it or to open a **pre-filled GitHub issue** with the diagnostics attached.

**Packaging & CI:**
- `bundle.macOS.minimumSystemVersion` `11.0` + `bundle.category` productivity;
  `pnpm tauri build` produces a working **`.app` + `.dmg`** (aarch64).
- CI matrix extended to `macos-latest` (+ `windows-latest`) with a `fetch-models` step
  (tauri-build validates `bundle.resources` on every `cargo check`).

## Next

Deferred: **code signing + notarization** (needs an Apple Developer account) for distribution
beyond a local machine; Intel/universal binary (arm64-only ships today); auto-update, batch
processing.
