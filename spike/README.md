# Фаза 0 — спайк валидации ядра

Цель: до постройки Tauri-оболочки убедиться на **реальном файле и реальном
(желательно слабом) железе**, что нативное ONNX-ядро даёт приемлемые скорость/RAM
и качество диаризации. Никакого Rust, ffmpeg или облака — только Python + готовые
wheels `sherpa-onnx` и PyAV для декода.

## Установка

```bash
cd spike
python -m venv .venv
.venv\Scripts\activate           # Windows (PowerShell: .venv\Scripts\Activate.ps1)
# source .venv/bin/activate      # macOS/Linux
pip install -r requirements.txt
```

## Загрузка моделей

```bash
# Русский (приоритет): GigaAM v2 CTC (MIT, ~226 МБ) + модели диаризации (~45 МБ)
python download_models.py --engine gigaam --diarization

# Многоязычный фолбэк:
python download_models.py --engine whisper-small
# Мультиязычный (25 EU языков, тег может отличаться — см. коммент в скрипте):
python download_models.py --engine parakeet
```

Модели кладутся в `spike/models/` (в git не коммитятся).

## Запуск

```bash
# транскрипция + диаризация русского файла
python run_spike.py --audio "C:\путь\к\записи.mp3" --engine gigaam --diarize

# только транскрипция, форсировать число спикеров, машинный вывод
python run_spike.py --audio meeting.mp4 --engine gigaam --diarize --num-speakers 3 --json
```

## Что меряем (Exit-критерии Фазы 0)

- **RTF** и «×realtime» для ASR и диаризации → подтвердить «часы → минуты» на слабом ноуте.
- **Пиковая RAM** — влезает ли в 8 ГБ (особенно диаризация поверх ASR).
- **Качество спикеров** — сверить разметку с текущим прод-воркером на 2–3 общих файлах.
- Выбрать набор моделей по умолчанию под разное железо/язык.

## Заметки

- Пути к файлам моделей ищутся glob-ом внутри `models/` — если структура распакованного
  архива иная, поправь паттерны в `find(...)` внутри `run_spike.py`.
- API sherpa-onnx для сборки распознавателя/диаризатора проверяется на первом запуске;
  если фабрика назовётся иначе в установленной версии — это ожидаемая точка правки Фазы 0.
- Parakeet-v3 ONNX-тег в релизах меняется — при 404 сверься с зоопарком sherpa-onnx
  (ссылка в `download_models.py`).
