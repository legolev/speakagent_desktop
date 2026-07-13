<div align="center">

<a href="README.md">Русский</a> · <a href="README.en.md"><b>English</b></a>

<img src="assets/banner.svg" alt="SpeakAgent Desktop" width="100%">

<h3>Transcription, speaker labels, and meeting notes — 100% offline, on any PC</h3>

<p>
A desktop app that turns any audio or video into a clean, speaker-labelled transcript,
then — with a local LLM — into a summary, protocol, or to-do list. Everything runs on
your machine: not a single byte of the recording ever leaves it.
</p>

<p>
<a href="../../releases/latest"><img src="https://img.shields.io/github/v/release/legolev/speakagent_desktop?style=flat&color=f59e0b&label=release" alt="Latest release"></a>
<a href="../../actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/legolev/speakagent_desktop/ci.yml?style=flat&label=CI" alt="CI"></a>
<img src="https://img.shields.io/badge/Windows-10%2F11-0a0a0b?style=flat&logo=windows11&logoColor=fbbf24" alt="Windows 10/11">
<img src="https://img.shields.io/badge/macOS-Intel%20%2B%20Apple%20Silicon-0a0a0b?style=flat&logo=apple&logoColor=white" alt="macOS Intel + Apple Silicon">
<img src="https://img.shields.io/badge/100%25-offline-16a34a?style=flat" alt="100% offline">
<img src="https://img.shields.io/badge/license-PolyForm%20NC%201.0.0-f59e0b?style=flat" alt="PolyForm NC license">
<img src="https://img.shields.io/badge/Tauri-2-fbbf24?style=flat&logo=tauri&logoColor=black" alt="Tauri 2">
</p>

</div>

---

## Table of contents

[Features](#-features) ·
[Screenshots](#-screenshots) ·
[How it works](#️-how-it-works) ·
[Models](#-models) ·
[Performance](#-performance) ·
[Install](#-install) ·
[Tech stack](#-tech-stack) ·
[Roadmap](#️-roadmap) ·
[License](#-license)

## ✨ Features

| | |
|---|---|
| 🔒 **Fully offline** | Transcription, diarization, and AI notes run locally. Audio never leaves the machine — ideal for interviews, negotiations, and medical/legal/HR recordings. |
| 🗣️ **Who spoke when** | Speech is split by speaker automatically: “Speaker 1 [00:14]: …”. Set the exact speaker count or let it auto-detect. |
| 📝 **Meeting notes** | A local LLM turns the transcript into a summary, meeting minutes, interview digest, or to-do list — with interactive checkboxes. |
| 💻 **Runs on low-end hardware** | CPU-first: even on a single core it's ~7× faster than real time (~17 min per hour of audio), peak RAM < 1 GB. No GPU required. |
| 🌍 **Model & language choice** | Russian by default (GigaAM v3), plus multilingual Parakeet (25 languages) and Whisper — from small to large-v3 turbo (99 languages) — switchable in settings. |
| 🎧 **Karaoke-highlight player** | A built-in audio/video player highlights the active line; click a line to seek to it. |
| ✏️ **Speaker renaming** | “Speaker 1” → “John”: names persist and are even injected into LLM prompts, so the minutes say “John”, not “Speaker 2”. |
| 📄 **Export TXT / MD / PDF** | Export transcripts and notes; PDF is generated natively with an embedded Cyrillic-capable font. |
| 🕘 **Recording history** | Results are stored in a local SQLite database and restored on the next launch. Auto-title from the meeting content. |
| ⚡ **GPU acceleration for notes** | A discrete Vulkan GPU (≥ 4 GB VRAM) speeds the LLM up ~10× automatically; on an iGPU / without Vulkan it silently falls back to CPU. |
| 📥 **No-surprise downloads** | A first-run wizard and background downloads with real sizes shown in the UI; small models are embedded in the installer. |
| 🖱️ **Drag & drop, cancel, ETA** | File drag-and-drop, a Stop button, self-calibrating time estimates, and a live status bar with CPU/RAM load. |
| 🎙️ **Dictation (push-to-talk)** | A global hotkey (even a single key — e.g. right Shift): hold → speak, release → text is recognized locally, copied to the clipboard and pasted at the cursor in any app. A history of your dictations on its own tab. |
| 🔌 **Local MCP server** | A built-in Model Context Protocol server on `127.0.0.1`: any AI/code agent (Claude Code, Cursor, VS Code, Codex) can drive the engine — transcribe files, diarize, generate minutes, browse history. One-click add-to-client buttons. |
| 🖥️ **Tray + background** | Minimizes to the tray (system-style icon); closing the window doesn't stop processing or the MCP server. |
| ⬆️ **Auto-update** | About → "Check": the app finds a newer GitHub release, downloads and installs it seamlessly (signed updates). |

## 🎬 Screenshots

<p align="center">
  <img src="assets/screenshots/demo-karaoke.gif" width="88%" alt="Speaker-labelled dialogue with timestamps; the active line highlights during playback">
</p>
<p align="center"><sub>A speaker-labelled transcript — the active line highlights in sync with the player, and clicking a line seeks to it.</sub></p>

<table>
  <tr>
    <td width="50%" align="center"><img src="assets/screenshots/01-home.png" alt="Home"><br><sub>Home</sub></td>
    <td width="50%" align="center"><img src="assets/screenshots/03-dictation.png" alt="Dictation"><br><sub>Dictation (push-to-talk)</sub></td>
  </tr>
  <tr>
    <td align="center"><img src="assets/screenshots/04-mcp.png" alt="MCP server"><br><sub>Local MCP server</sub></td>
    <td align="center"><img src="assets/screenshots/05-about.png" alt="About"><br><sub>About · auto-update</sub></td>
  </tr>
</table>

## ⚙️ How it works

One native C++/ONNX pipeline, entirely on your machine:

```mermaid
flowchart LR
    A["Audio / video"] -->|"symphonia · ffmpeg"| B["16 kHz mono PCM"]
    B -->|"Silero VAD"| C["Speech segments"]
    C -->|"GigaAM / Parakeet / Whisper"| D["Words + timestamps"]
    C -->|"pyannote + CAM++"| E["Who spoke when"]
    D --> F["Post-processing"]
    E --> F
    F --> G["Speaker-labelled dialogue"]
    G -.->|"local LLM"| H["Meeting notes"]
```

1. **Decode** — audio/video is decoded to 16 kHz mono PCM: natively via `symphonia`, with an `ffmpeg` fallback for webm/opus and anything symphonia can't handle.
2. **Speech chunking (VAD)** — Silero VAD cuts long files into segments (≤ 20 s) so boundaries land in silence, never mid-word.
3. **Recognition (ASR)** — each segment goes through sherpa-onnx (GigaAM CTC / Parakeet / Whisper) → words with timestamps; GigaAM v3 emits punctuation and casing directly.
4. **Diarization** — pyannote-segmentation + a CAM++ embedder determine “who spoke when” (clustering threshold, or an exact speaker count from the UI).
5. **Post-processing** — words are assigned to speakers by maximum overlap, islands are smoothed, and adjacent turns are merged → “Speaker N [timestamp]: text”.
6. **Meeting notes** (optional) — the transcript goes to a local LLM via `llama-server` → summary / protocol / to-dos. Long recordings use map-reduce with a cached digest, so a second artifact for the same meeting is ~9× cheaper.

## 🧠 Models

The language model is chosen in settings; the supporting diarization models download in the
background. Everything is **downloaded on first run** and is not part of this repository.

| Model | Role | Size | License |
|---|---|---:|---|
| **GigaAM v3 CTC-punct** | ASR — Russian (default), punctuation + word timestamps | ~160 MB | ⚠️ Noncommercial |
| **Parakeet-TDT-0.6b-v3** | ASR — 25 languages, fastest | ~640 MB | CC-BY-4.0 |
| **Whisper small** | ASR — 98 languages | ~466 MB | MIT |
| **Whisper large-v3 turbo** | ASR — 99 languages, for hard audio | ~540 MB | MIT |
| **Qwen3-4B-Instruct-2507** (Q4_K_M) | LLM notes — best quality | ~2.4 GB | Apache-2.0 |
| **Qwen3-1.7B** (Q4_K_M) | LLM notes — for low-end PCs | ~1.1 GB | Apache-2.0 |
| pyannote-segmentation-3.0 | Diarization — speech segmentation | ~6 MB | MIT |
| CAM++ (3D-Speaker) | Diarization — voice embedding | ~28 MB | Apache-2.0 |
| Silero VAD | Voice activity detection | ~2 MB | MIT |

> ⚠️ The default **GigaAM** model is under a noncommercial license. Full component
> licensing is in [`NOTICE.md`](NOTICE.md).

## 📊 Performance

CPU measurements (Phase 0 and real meetings). A GPU only helps with the notes — it doesn't speed up ASR.

| Metric | Value |
|---|---|
| GigaAM (Russian), CPU | **×15.9** faster than real time |
| Parakeet-v3, CPU | ×24 |
| Whisper-small, CPU | ×2.7 |
| Diarization, CPU | ×14 |
| ASR + diarization, 1 hour of audio | ~8–9 min (multi-core CPU) |
| Even on 1 core | ×7 (~17 min per hour) |
| Peak RAM | **< 1 GB**, does not grow with length |
| Installer / release exe | ~8 MB / ~32 MB |
| Notes on CPU (warm server) | summary ~33 s |
| Notes on GPU (Vulkan, RTX 3090 Ti) | ~169 tok/s — roughly ×10 over CPU |

## 🚀 Install

### Prebuilt installer

Download the latest installer from [**Releases**](../../releases):
- **macOS Apple Silicon (M1+)** — `*_aarch64.dmg`;
- **macOS Intel** — `*_x64.dmg`;
- **Windows 10/11** — `*-setup.exe` (NSIS).

An installed app **updates itself**: About → "Check" (or a silent check when the page opens)
finds a new GitHub release and installs it seamlessly.

> **First launch** (the app isn't code-signed with a paid Apple/Microsoft certificate yet):
> - **macOS** — drag it to Applications, then **right-click the app → "Open"** → "Open".
>   If macOS says it's "damaged", clear the quarantine once:
>   `xattr -cr /Applications/SpeakAgent.app`
> - **Windows** — on the SmartScreen prompt click "More info" → "Run anyway".
>
> After that the app updates itself — no more warnings.

### Build from source

**Requires:** [Rust](https://rustup.rs/) (`stable-x86_64-pc-windows-msvc`) + **MSVC C++
Build Tools** (linker + Windows SDK) + [Node 20](https://nodejs.org/) and
[pnpm](https://pnpm.io/). FFmpeg is downloaded automatically — no system install needed.

```bash
pnpm install
pnpm tauri dev                       # run the app (Vite + native window)
pnpm tauri build --bundles nsis      # installer → src-tauri/target/release/bundle/nsis/*.exe
```

MSVC Build Tools (if missing, from an elevated terminal):

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

**Quick headless engine test** (verify the Rust core without a GUI):

```bash
cd src-tauri
cargo run --example try_transcribe -- "<file>" [secs] [diarize]
cargo run --example try_llm -- gen <transcript.txt> [summary|business|interview|todo]
```

### Releases (for maintainers)

Building `.dmg`/`.exe` and publishing to Releases is automated via GitHub Actions
([`.github/workflows/release.yml`](.github/workflows/release.yml)):

```bash
# 1) bump "version" in src-tauri/tauri.conf.json and package.json (e.g. 0.1.2)
# 2) commit and push a tag:
git tag v0.1.2 && git push origin v0.1.2
```

CI builds macOS (arm64) and Windows, signs the update artifacts, and creates a Release with
installers + `latest.json` (which drives auto-update).

**Repository secrets** (Settings → Secrets → Actions):

_Update signing_ (Ed25519, `pnpm tauri signer generate`; the public key is already in
`tauri.conf.json → plugins.updater.pubkey`):
- `TAURI_SIGNING_PRIVATE_KEY` — the private key;
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — the key password (empty if none).

_macOS code signing_ (a self-signed certificate — a stable "identity" so macOS permissions
don't reset across updates; CI adds trust to it automatically):
- `APPLE_CERTIFICATE` — the `.p12` in base64;
- `APPLE_CERTIFICATE_PASSWORD` — the `.p12` password;
- `APPLE_SIGNING_IDENTITY` — the certificate's Common Name (e.g. `SpeakAgent`);
- `KEYCHAIN_PASSWORD` — any password for the temporary CI keychain.

⚠️ Keep private keys/certificates secret. Without the macOS secrets the build is unsigned
(ad-hoc) — it works, but permissions reset on every update. For a fully seamless launch (no
Gatekeeper warnings) you need a paid Apple Developer ID + notarization.

## 🛠 Tech stack

- **Shell:** Tauri 2 (Rust core + system WebView). ~8 MB installer; ~32 MB release exe with sherpa-onnx + ONNX Runtime statically linked — no side DLLs.
- **Frontend:** React 19 · Vite 7 · Tailwind 4 · Zustand · TanStack Query · react-router. Dark zinc + amber theme, glass/vibrancy.
- **Engine:** [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) 1.13.4 (official Rust crate, prebuilt binaries — no C++ build). ASR + diarization + Silero VAD.
- **Local LLM:** `llama-server` (llama.cpp) as a sidecar, downloaded at runtime like ffmpeg; Vulkan build (GPU + CPU in one archive); OpenAI-compatible HTTP over `ureq` with streaming.
- **Decode:** `symphonia` (pure Rust) → `ffmpeg` fallback. **Storage:** SQLite (`rusqlite`). **PDF:** `genpdf` + embedded DejaVu.

## 🗺️ Roadmap

| Phase | Scope | Status |
|---|---|---|
| 0 | Validate the core (sherpa-onnx) on real files | ✅ Done |
| 1 | Windows MVP: transcription + diarization, models, export, history, player | ✅ Done |
| 2 | macOS (Intel + Apple Silicon) + Metal: `.app`/`.dmg`, vibrancy | ✅ Done |
| 3 | Meeting notes — minutes/summaries/to-dos via a local LLM | ✅ Done |
| — | Dictation (push-to-talk), local MCP server, tray | ✅ Done |
| — | Auto-update (GitHub Releases, signed updates) | ✅ Done |
| 4 | Monetization / licensing, code signing (Gatekeeper/SmartScreen) | ⬜ Planned |
| 5+ | Folder watch, subtitle editor, glossary, i18n, live streaming | ⬜ Planned |

See [`CHANGELOG.md`](CHANGELOG.md) for the full history.

## 📄 License

The code is licensed under [**PolyForm Noncommercial 1.0.0**](LICENSE.md): free to study,
modify, and use for **noncommercial** purposes. Third-party components and models keep
their own licenses — notably, **the default Russian GigaAM model is noncommercial**.
The full list is in [`NOTICE.md`](NOTICE.md).

## 🙏 Acknowledgements

This project stands on the shoulders of open components:
[sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx),
[llama.cpp](https://github.com/ggml-org/llama.cpp),
[GigaAM](https://github.com/salute-developers/GigaAM),
[Whisper](https://github.com/openai/whisper),
[NVIDIA NeMo Parakeet](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3),
[pyannote](https://github.com/pyannote/pyannote-audio),
[Silero VAD](https://github.com/snakers4/silero-vad),
[Tauri](https://github.com/tauri-apps/tauri) — thanks to their authors.

<div align="center">
<sub>Made with ❤️ for people who value the privacy of their recordings.</sub>
</div>
