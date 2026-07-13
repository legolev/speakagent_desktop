# Как внести вклад / Contributing

Спасибо за интерес к SpeakAgent Desktop! Обсуждать задачи можно на русском или английском.
_You're welcome to open issues and PRs in Russian or English._

## Требования

- [Rust](https://rustup.rs/) (стабильный; целевая платформа берётся по хосту — см. `rust-toolchain.toml`)
- **Windows:** MSVC C++ Build Tools (линкер + Windows SDK)
- **macOS:** Xcode Command Line Tools (`xcode-select --install`) — **актуальной версии**:
  старые (clang 12 / SDK 11.3) не линкуют prebuilt onnxruntime (нет символов libc++
  `std::filesystem`/`fstream`)
- [Node 20](https://nodejs.org/) (см. `.nvmrc`) + [pnpm](https://pnpm.io/)

FFmpeg и языковые модели докачиваются приложением при первом запуске — ставить их вручную не нужно.

## Запуск и сборка

```bash
pnpm install
pnpm tauri dev                       # приложение (Vite + нативное окно)
pnpm build                           # сборка фронтенда (tsc + vite)
pnpm tauri build --bundles nsis      # Windows: установщик (.exe)
pnpm tauri build --bundles dmg       # macOS: .app + .dmg
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
[Apache License 2.0](LICENSE.md), под которой опубликован проект.

## Релизы (для мейнтейнеров)

Сборка `.dmg`/`.exe` и публикация в GitHub Releases — автоматически по тегу `v*`
([`.github/workflows/release.yml`](.github/workflows/release.yml)):

```bash
# 1) поднять "version" в src-tauri/tauri.conf.json и package.json (напр. 0.1.3)
# 2) закоммитить и запушить тег:
git tag v0.1.3 && git push origin v0.1.3
```

CI собирает macOS (Apple Silicon + Intel) и Windows, подписывает артефакты обновления
и создаёт Release с установщиками + `latest.json` (по нему работает автообновление).

**Секреты репозитория** (Settings → Secrets → Actions):

- **Подпись обновлений** (Ed25519, `pnpm tauri signer generate`; публичный ключ уже
  в `tauri.conf.json → plugins.updater.pubkey`):
  `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- **Подпись кода macOS** (самоподписанный сертификат — стабильная «личность», чтобы
  разрешения macOS не слетали при обновлениях):
  `APPLE_CERTIFICATE` (.p12 в base64), `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY` (Common Name, напр. `SpeakAgent`), `KEYCHAIN_PASSWORD`.

⚠️ Приватные ключи/сертификаты — в тайне, не в репозитории. Без macOS-секретов сборка
идёт без подписи (ad-hoc): работает, но разрешения сбрасываются на каждом обновлении.
Для полностью бесшовного запуска (без предупреждений Gatekeeper) нужен платный Apple
Developer ID + нотаризация.
