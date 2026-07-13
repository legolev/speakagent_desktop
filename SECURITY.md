# Security Policy / Безопасность

## Supported versions / Поддерживаемые версии

Security fixes land in the **latest release**. Please update before reporting
(«О приложении» → «Проверить»).

Исправления безопасности выходят в **последнем релизе**. Перед обращением, пожалуйста,
обновитесь.

## Reporting a vulnerability / Как сообщить об уязвимости

**Do not open a public issue for security problems.**

Please report privately via GitHub's **[private vulnerability reporting](https://github.com/legolev/speakagent_desktop/security/advisories/new)**
(repo → **Security** → **Report a vulnerability**). Include steps to reproduce, affected
version/OS, and impact. We aim to acknowledge within a few days; timelines are best-effort
for this noncommercial side project.

**Не открывайте публичный issue по проблемам безопасности.** Сообщайте приватно через
**Security → Report a vulnerability** в репозитории. Укажите шаги воспроизведения, версию
и ОС, характер воздействия.

## Scope notes / Область

SpeakAgent is **offline-first**: audio never leaves the machine. Worth extra scrutiny:

- the local **MCP server** (JSON-RPC over HTTP, bound to `127.0.0.1`, optional bearer token);
- the **auto-updater** (updates are signed; the public key ships in `tauri.conf.json`);
- locally stored settings (including an optional cloud-AI provider token, if you enable one).
