# Third-party notices / Сторонние компоненты

SpeakAgent Desktop's own source code is licensed under the **PolyForm Noncommercial
License 1.0.0** (see [`LICENSE.md`](LICENSE.md)). The application builds on, bundles,
or downloads at runtime the third-party components listed below, each under **its own
license**. This file is provided for attribution and to make those obligations visible.

> **Собственный код** приложения — под PolyForm Noncommercial 1.0.0. Перечисленные ниже
> сторонние компоненты остаются под своими лицензиями.

---

## ⚠️ Важно про модели / Model licensing

Модели **скачиваются при первом запуске** и **не входят** в исходный код или установщик
этого репозитория. Каждая модель подчиняется собственной лицензии.

**Модель русского языка по умолчанию — GigaAM — распространяется под некоммерческой
лицензией.** В сочетании с некоммерческой лицензией самого приложения это значит, что
SpeakAgent Desktop в конфигурации по умолчанию предназначен для **некоммерческого**
использования. Для коммерческих сценариев проверьте лицензии конкретных моделей
(например, замените модель ASR на такую, чья лицензия это позволяет) и лицензию приложения.

---

## Библиотеки сборки / Build-time libraries

| Компонент | Роль | Лицензия |
|---|---|---|
| [Tauri 2](https://github.com/tauri-apps/tauri) | Оболочка приложения (Rust + WebView) | Apache-2.0 / MIT |
| [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) | ASR, диаризация, VAD | Apache-2.0 |
| [ONNX Runtime](https://github.com/microsoft/onnxruntime) | Инференс ONNX (статически) | MIT |
| [tract](https://github.com/sonos/tract) | Инференс ONNX для RUPunct (чистый Rust) | Apache-2.0 / MIT |
| [tokenizers](https://github.com/huggingface/tokenizers) | WordPiece-токенизатор | Apache-2.0 |
| [Symphonia](https://github.com/pdeljanov/Symphonia) | Декод аудио (чистый Rust) | MPL-2.0 |
| [rusqlite](https://github.com/rusqlite/rusqlite) / SQLite | Локальная БД истории | MIT / Public Domain |
| [genpdf](https://github.com/Techassi/genpdf-rs) | Генерация PDF | Apache-2.0 / MIT |
| [ureq](https://github.com/algesten/ureq) | HTTP-клиент к llama-server | Apache-2.0 / MIT |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | Детект железа | MIT |
| React · Vite · Tailwind CSS · Zustand · TanStack Query · react-markdown · lucide-react | Фронтенд | MIT |

## Загружаемое в рантайме / Runtime-downloaded binaries

| Компонент | Роль | Лицензия |
|---|---|---|
| [llama.cpp / llama-server](https://github.com/ggml-org/llama.cpp) | Движок локальной LLM (sidecar) | MIT |
| [FFmpeg](https://ffmpeg.org/) (сборка [gyan.dev](https://www.gyan.dev/ffmpeg/builds/)) | Fallback-декод медиа | GPL / LGPL |

## Модели / Models

| Модель | Роль | Лицензия |
|---|---|---|
| [GigaAM v3 CTC-punct](https://github.com/salute-developers/GigaAM) | ASR (русский, по умолчанию) | **Некоммерческая** (см. карточку модели) |
| [Parakeet-TDT-0.6b-v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) (NVIDIA NeMo) | ASR (25 языков) | CC-BY-4.0 |
| [Whisper small](https://github.com/openai/whisper) (OpenAI) | ASR (98 языков) | MIT |
| [Qwen3-4B-Instruct-2507](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507) / [Qwen3-1.7B](https://huggingface.co/Qwen/Qwen3-1.7B) | LLM «Итоги встречи» | Apache-2.0 |
| [pyannote segmentation 3.0](https://huggingface.co/pyannote/segmentation-3.0) | Диаризация (сегментация) | MIT |
| [CAM++ (3D-Speaker)](https://github.com/modelscope/3D-Speaker) | Диаризация (эмбеддинг голосов) | Apache-2.0 |
| [Silero VAD](https://github.com/snakers4/silero-vad) | Детектор речи | MIT |
| [RUPunct small](https://huggingface.co/ekhodzitsky/rupunct-small-onnx) | Пунктуация (дремлющий fallback) | MIT |

## Шрифты / Fonts

| Компонент | Роль | Лицензия |
|---|---|---|
| [DejaVu Sans](https://dejavu-fonts.github.io/) | Кириллический шрифт для PDF-экспорта | Bitstream Vera / Public Domain (permissive) |

---

Названия и лицензии приведены на основе карточек и репозиториев компонентов на момент
написания; при использовании сверяйтесь с актуальными условиями по ссылкам выше.
Licenses are stated per each component's model card / repository at the time of writing —
always verify against the upstream terms linked above.
