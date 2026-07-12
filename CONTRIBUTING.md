# Как внести вклад / Contributing

Спасибо за интерес к SpeakAgent Desktop! Обсуждать задачи можно на русском или английском.
_You're welcome to open issues and PRs in Russian or English._

## Требования

- [Rust](https://rustup.rs/) — `stable-x86_64-pc-windows-msvc` (см. `rust-toolchain.toml`)
- **MSVC C++ Build Tools** (линкер + Windows SDK)
- [Node 20](https://nodejs.org/) (см. `.nvmrc`) + [pnpm](https://pnpm.io/)

FFmpeg и языковые модели докачиваются приложением при первом запуске — ставить их вручную не нужно.

## Запуск и сборка

```bash
pnpm install
pnpm tauri dev                       # приложение (Vite + нативное окно)
pnpm build                           # сборка фронтенда (tsc + vite)
pnpm tauri build --bundles nsis      # установщик
```

## Быстрый тест движка без GUI

Самый быстрый способ проверить Rust-ядро — headless-примеры:

```bash
cd src-tauri
cargo run --example try_transcribe -- "<файл>" [сек] [diarize]
cargo run --example try_llm -- gen <transcript.txt> [summary|business|interview|todo]
cargo run --example try_punct -- "текст без пунктуации"
```

## Структура и правила

- **Универсальное ядро** живёт в `src-tauri/src/engine/*` и должно оставаться **OS-agnostic**.
  Платформенное (окно, IPC, vibrancy) — только в `src-tauri/src/lib.rs`. Держите эту границу.
- Тяжёлые задачи — вне UI-потока (`spawn_blocking`), прогресс через Tauri `Channel`.
- Состояние задач — в zustand-сторе, не в страницах (переживает навигацию).
- Пользовательские тексты — на русском и нетехнические (названия моделей — только в настройках).

## Стиль кода

- **Rust:** `cargo fmt` перед коммитом; по возможности без предупреждений `cargo clippy`.
- **TypeScript:** сборка должна проходить `tsc` без ошибок.
- Комментарии и коммиты — по-русски или по-английски, по смыслу.

## Pull requests

1. Форкните репозиторий и создайте ветку от `main`.
2. Убедитесь, что `pnpm build` и `cargo check` (в `src-tauri`) проходят.
3. Обновите `CHANGELOG.md`, если поменяли поведение.
4. Опишите, что и зачем; приложите шаги проверки.

## Лицензия вклада

Отправляя PR, вы соглашаетесь, что ваш вклад распространяется на условиях
[PolyForm Noncommercial License 1.0.0](LICENSE.md), под которой опубликован проект.
