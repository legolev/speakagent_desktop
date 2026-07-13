//! Менеджер моделей: каталог, загрузка в каталог данных, резолвинг путей.
//! Убирает хардкод — модели качаются при первом запуске, лежат под единым корнем данных.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::engine::asr::{AsrFiles, Engine};

#[derive(Clone, Copy, PartialEq)]
enum Format {
    TarBz2,
    SingleFile, // один файл → кладётся по пути `check`
    Zip,        // архив ffmpeg: вынимаем только сам бинарник
    FileSet,    // несколько файлов по своим относительным путям (см. Spec::files)
    ZipDir,     // zip с набором файлов (llama, Windows) → в каталог рядом с `check`
    TarGzDir,   // то же из .tar.gz (llama, macOS)
}

pub struct Spec {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: &'static str, // "asr" | "diarization" | "tool" | "llm"
    pub lang: &'static str,
    url: &'static str,   // URL для Windows; пер-ОС ветвление — в Spec::url()
    format: Format,      // формат для Windows; пер-ОС ветвление — в Spec::format_os()
    check: &'static str, // путь относительно models_dir (Windows-вид); см. check_path()
    pub size_mb: u32,
    pub required: bool,
    files: &'static [(&'static str, &'static str)], // (url, rel_path) для Format::FileSet
}

/// Закреплённый релиз llama.cpp (sidecar «Помощника итогов»). Ассеты проверены.
const LLAMA_TAG: &str = "b9957";

impl Spec {
    /// URL под текущую ОС. Каталог объявлен в Windows-виде; для инструментов
    /// с платформенными сборками (ffmpeg, llama) здесь подменяется на macOS-ассет.
    fn url_os(&self) -> String {
        if cfg!(target_os = "macos") {
            match self.id {
                "ffmpeg" => {
                    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };
                    return format!(
                        "https://ffmpeg.martin-riedl.de/redirect/latest/macos/{arch}/release/ffmpeg.zip"
                    );
                }
                "llama" => {
                    return format!(
                        "https://github.com/ggml-org/llama.cpp/releases/download/{LLAMA_TAG}/llama-{LLAMA_TAG}-bin-macos-arm64.tar.gz"
                    );
                }
                _ => {}
            }
        }
        self.url.to_string()
    }

    /// Формат архива под текущую ОС (llama: zip на Windows, tar.gz на macOS).
    fn format_os(&self) -> Format {
        if self.id == "llama" && cfg!(target_os = "macos") {
            Format::TarGzDir
        } else {
            self.format
        }
    }

    /// `check` под текущую ОС: на unix у бинарников нет суффикса .exe.
    fn check_path(&self) -> String {
        if cfg!(windows) {
            self.check.to_string()
        } else {
            self.check.trim_end_matches(".exe").to_string()
        }
    }
}

/// Имя исполняемого файла под текущую ОС («ffmpeg» → «ffmpeg.exe» на Windows).
pub fn tool_exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Выставить exec-бит на unix (после распаковки архивы дают 0644 → «Permission denied»).
fn make_executable(_p: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(_p, std::fs::Permissions::from_mode(0o755));
    }
}

pub const CATALOG: &[Spec] = &[
    Spec {
        id: "gigaam",
        name: "Русский (GigaAM)",
        kind: "asr",
        lang: "русский",
        // v3 CTC-**punct**: точнее v2 + пунктуация и заглавные ВСТРОЕНЫ в модель (токены знаков),
        // при этом CTC даёт таймкоды слов → точная диаризация. Внешний RUPunct для неё не нужен.
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-ctc-punct-giga-am-v3-russian-2025-12-16.tar.bz2",
        format: Format::TarBz2,
        check: "sherpa-onnx-nemo-ctc-punct-giga-am-v3-russian-2025-12-16/model.int8.onnx",
        size_mb: 160,
        required: true,
        files: &[],
    },
    Spec {
        id: "diar-seg",
        name: "Разделение речи по фразам",
        kind: "diarization",
        lang: "—",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2",
        format: Format::TarBz2,
        check: "sherpa-onnx-pyannote-segmentation-3-0/model.onnx",
        size_mb: 6,
        required: true,
        files: &[],
    },
    Spec {
        id: "diar-emb",
        name: "Различение голосов",
        kind: "diarization",
        lang: "—",
        // Мультиязычный CAM++ (zh+en) — заметно лучше отделяет НЕ-китайские (русские)
        // голоса, чем прежний zh-cn. Та же архитектура/размерность — drop-in.
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh_en_16k-common_advanced.onnx",
        format: Format::SingleFile,
        check: "3dspeaker_speech_campplus_sv_zh_en_16k-common_advanced.onnx",
        size_mb: 28,
        required: true,
        files: &[],
    },
    Spec {
        id: "vad",
        name: "Детектор речи (VAD)",
        kind: "tool",
        lang: "—",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx",
        format: Format::SingleFile,
        check: "silero_vad.onnx",
        size_mb: 2,
        required: true,
        files: &[],
    },
    Spec {
        id: "punct-ru",
        name: "Пунктуация и заглавные буквы",
        kind: "tool",
        lang: "русский",
        // RUPunct small (int8) + WordPiece-токенизатор + config. Запускается через tract
        // (чистый Rust), не требует второго onnxruntime. Только для русского (GigaAM).
        url: "",
        format: Format::FileSet,
        check: "rupunct/rupunct_small_int8.onnx",
        size_mb: 32,
        // fallback-модель: качается только когда активна русская модель БЕЗ своей пунктуации
        // (сейчас таких нет — GigaAM v3 пунктуирует сам), поэтому не тянем её автоматически.
        required: false,
        files: &[
            (
                "https://huggingface.co/ekhodzitsky/rupunct-small-onnx/resolve/main/rupunct_small_int8.onnx",
                "rupunct/rupunct_small_int8.onnx",
            ),
            (
                "https://huggingface.co/ekhodzitsky/rupunct-small-onnx/resolve/main/tokenizer.json",
                "rupunct/tokenizer.json",
            ),
            (
                "https://huggingface.co/ekhodzitsky/rupunct-small-onnx/resolve/main/config.json",
                "rupunct/config.json",
            ),
        ],
    },
    Spec {
        id: "ffmpeg",
        name: "Конвертер медиа (ffmpeg)",
        kind: "tool",
        lang: "—",
        url: "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip",
        format: Format::Zip,
        check: "bin/ffmpeg.exe",
        size_mb: 98,
        required: true,
        files: &[],
    },
    Spec {
        id: "llama",
        name: "Помощник итогов (движок)",
        kind: "tool",
        lang: "—",
        // Прибилженный llama-server (sidecar по образцу ffmpeg): качается при первом
        // включении «Итогов», в сборку приложения не входит. Vulkan-сборка содержит
        // и все CPU-бэкенды: на машинах без GPU/Vulkan тихо работает на CPU —
        // один архив для всех. macOS-ассет — в url_os().
        url: "https://github.com/ggml-org/llama.cpp/releases/download/b9957/llama-b9957-bin-win-vulkan-x64.zip",
        format: Format::ZipDir,
        check: "bin/llama/llama-server.exe",
        size_mb: 32,
        required: false, // качается вместе с первой LLM-моделью (ensure_llm), не на старте
        files: &[],
    },
    Spec {
        id: "parakeet",
        name: "Мультиязычный (Parakeet)",
        kind: "asr",
        lang: "25 языков",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2",
        format: Format::TarBz2,
        check: "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/encoder.int8.onnx",
        size_mb: 640,
        required: false,
        files: &[],
    },
    Spec {
        id: "whisper-small",
        name: "Whisper small",
        kind: "asr",
        lang: "98 языков",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-whisper-small.tar.bz2",
        format: Format::TarBz2,
        check: "sherpa-onnx-whisper-small/small-encoder.int8.onnx",
        size_mb: 466,
        required: false,
        files: &[],
    },
    Spec {
        id: "whisper-turbo",
        name: "Whisper large-v3 turbo",
        kind: "asr",
        lang: "99 языков",
        // large-v3-turbo (в sherpa — «turbo»): устойчивее к шуму/музыке, чем GigaAM, но
        // ТЯЖЁЛЫЙ и МЕДЛЕННЫЙ на CPU (~1,6 ГБ в память). Для сильных машин / трудного аудио.
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-whisper-turbo.tar.bz2",
        format: Format::TarBz2,
        check: "sherpa-onnx-whisper-turbo/turbo-encoder.int8.onnx",
        size_mb: 540,
        required: false,
        files: &[],
    },
    // ── Модели «Итогов встречи» (локальный LLM, выбираются в Настройках) ──
    Spec {
        id: "llm-qwen3-4b",
        name: "Стандартная — лучшее качество",
        kind: "llm",
        lang: "русский+",
        url: "https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
        format: Format::SingleFile,
        check: "llm/Qwen3-4B-Instruct-2507-Q4_K_M.gguf",
        size_mb: 2382,
        required: false,
        files: &[],
    },
    Spec {
        id: "llm-qwen3-17b",
        name: "Быстрая — для слабых компьютеров",
        kind: "llm",
        lang: "русский+",
        url: "https://huggingface.co/unsloth/Qwen3-1.7B-GGUF/resolve/main/Qwen3-1.7B-Q4_K_M.gguf",
        format: Format::SingleFile,
        check: "llm/Qwen3-1.7B-Q4_K_M.gguf",
        size_mb: 1056,
        required: false,
        files: &[],
    },
];

pub fn models_dir() -> PathBuf {
    let d = crate::engine::store::data_dir().join("models");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn find(id: &str) -> Option<&'static Spec> {
    CATALOG.iter().find(|s| s.id == id)
}

/// «Установлено» = резолвится где угодно (данные / вшитые в установщик / dev).
/// Благодаря этому missing_core() не перекачивает то, что уже в комплекте.
pub fn is_installed(spec: &Spec) -> bool {
    resolve(&spec.check_path()).is_some()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub lang: String,
    pub size_mb: u32,
    pub required: bool,
    pub installed: bool,
}

pub fn list() -> Vec<ModelInfo> {
    CATALOG
        .iter()
        .map(|s| ModelInfo {
            id: s.id.into(),
            name: s.name.into(),
            kind: s.kind.into(),
            lang: s.lang.into(),
            size_mb: s.size_mb,
            required: s.required,
            installed: is_installed(s),
        })
        .collect()
}

/// Потоковая загрузка URL в файл с прогрессом (следует за редиректами — как GitHub/HF LFS).
fn fetch_file(url: &str, dest: &Path, mut on: impl FnMut(u64, u64)) -> Result<(), String> {
    let resp = ureq::get(url).call().map_err(|e| format!("network error: {e}"))?;
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut reader = resp.into_reader();
    let mut f = File::create(dest).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1 << 16];
    let mut done: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        on(done, total);
    }
    Ok(())
}

pub fn download(id: &str, mut on_progress: impl FnMut(u64, u64)) -> Result<(), String> {
    let spec = find(id).ok_or("unknown model")?;
    let dir = models_dir();

    // ── мультифайловая модель (onnx + tokenizer + config): каждый файл по своему пути ──
    if spec.format == Format::FileSet {
        let n = spec.files.len().max(1) as u64;
        for (i, (url, rel)) in spec.files.iter().enumerate() {
            let dest = dir.join(rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            // прогресс — грубо по числу файлов (этот путь используется молча в фоне)
            on_progress(i as u64, n);
            fetch_file(url, &dest, |_, _| {})?;
        }
        on_progress(n, n);
        if !is_installed(spec) {
            return Err("model file not found after download".into());
        }
        return Ok(());
    }

    // ── одиночный файл/архив ──
    let tmp = dir.join(format!(".{}.part", spec.id));
    fetch_file(&spec.url_os(), &tmp, &mut on_progress)?;

    // ── распаковка / размещение ──
    match spec.format_os() {
        Format::TarBz2 => {
            let f = File::open(&tmp).map_err(|e| e.to_string())?;
            let bz = bzip2::read::BzDecoder::new(f);
            tar::Archive::new(bz)
                .unpack(&dir)
                .map_err(|e| format!("extraction: {e}"))?;
        }
        Format::Zip => {
            // ffmpeg: из Windows-архива берём bin/ffmpeg.exe, из macOS-архива — просто ffmpeg.
            let want = tool_exe("ffmpeg");
            let f = File::open(&tmp).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
            let mut placed = false;
            for i in 0..zip.len() {
                let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
                let name = file.name().replace('\\', "/");
                let base = name.rsplit('/').next().unwrap_or("");
                if base == want && !name.ends_with('/') {
                    std::fs::create_dir_all(dir.join("bin")).map_err(|e| e.to_string())?;
                    let dest = dir.join("bin").join(&want);
                    let mut out = File::create(&dest).map_err(|e| e.to_string())?;
                    std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
                    drop(out);
                    make_executable(&dest);
                    placed = true;
                    break;
                }
            }
            if !placed {
                return Err(format!("{want} not found in the archive"));
            }
        }
        // llama: из архива берём сервер + библиотеки + LICENSE в bin/llama/ (плоско).
        Format::ZipDir | Format::TarGzDir => {
            let dest_dir = dir.join("bin").join("llama");
            std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
            let keep = |base: &str| {
                base == tool_exe("llama-server")
                    || base == "LICENSE"
                    || base.ends_with(".dll")
                    || base.ends_with(".dylib")
                    || base.ends_with(".so")
            };
            if spec.format_os() == Format::ZipDir {
                let f = File::open(&tmp).map_err(|e| e.to_string())?;
                let mut zip = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
                for i in 0..zip.len() {
                    let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
                    let name = file.name().replace('\\', "/");
                    if name.ends_with('/') {
                        continue;
                    }
                    let base = name.rsplit('/').next().unwrap_or("").to_string();
                    if keep(&base) {
                        let dest = dest_dir.join(&base);
                        let mut out = File::create(&dest).map_err(|e| e.to_string())?;
                        std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
                        drop(out);
                        make_executable(&dest);
                    }
                }
            } else {
                let f = File::open(&tmp).map_err(|e| e.to_string())?;
                let gz = flate2::read::GzDecoder::new(f);
                let mut ar = tar::Archive::new(gz);
                for entry in ar.entries().map_err(|e| e.to_string())? {
                    let mut entry = entry.map_err(|e| e.to_string())?;
                    let et = entry.header().entry_type();
                    let path = entry.path().map_err(|e| e.to_string())?.into_owned();
                    let base = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    if base.is_empty() || !keep(&base) {
                        continue;
                    }
                    let dest = dest_dir.join(&base);
                    if et.is_symlink() {
                        // macOS-сборка llama содержит версионные симлинки
                        // (libX.0.dylib → libX.0.0.9957.dylib); dyld грузит по
                        // @rpath/libX.0.dylib, поэтому симлинк обязателен. Пересоздаём
                        // плоско — по basename цели (все дилибы лежат в одной папке).
                        #[cfg(unix)]
                        if let Ok(Some(target)) = entry.link_name().map(|o| o.map(|p| p.into_owned())) {
                            if let Some(tgt) = target.file_name().and_then(|s| s.to_str()) {
                                let _ = std::fs::remove_file(&dest);
                                let _ = std::os::unix::fs::symlink(tgt, &dest);
                            }
                        }
                    } else if et.is_file() {
                        let mut out = File::create(&dest).map_err(|e| e.to_string())?;
                        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                        drop(out);
                        make_executable(&dest);
                    }
                }
            }
        }
        Format::SingleFile => {
            // кладём по пути `check` (обратная совместимость: у старых записей check == имя файла)
            let dest = dir.join(spec.check_path());
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
        }
        Format::FileSet => unreachable!(),
    }
    let _ = std::fs::remove_file(&tmp);

    if !is_installed(spec) {
        return Err("model file not found after download".into());
    }
    Ok(())
}

/// Модели, вшитые в установщик (bundle.resources → рядом с exe). None, если нет.
fn bundled_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    #[cfg(target_os = "macos")]
    {
        // .app: Contents/MacOS/exe → Contents/Resources/models
        let r = dir.join("../Resources/models");
        if r.exists() {
            return Some(r);
        }
    }
    // Windows NSIS ($INSTDIR) и cargo target/: ресурсы лежат рядом с exe.
    // НЕ canonicalize: \\?\-пути ломают C-API sherpa.
    let r = dir.join("models");
    r.exists().then_some(r)
}

// ── резолвинг: каталог данных → вшитые в установщик → dev-фолбэк (debug) ──
fn resolve(rel: &str) -> Option<PathBuf> {
    let p = models_dir().join(rel);
    if p.exists() {
        return Some(p);
    }
    if let Some(b) = bundled_dir() {
        let p = b.join(rel);
        if p.exists() {
            return Some(p);
        }
    }
    #[cfg(debug_assertions)]
    {
        let dev = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../spike/models")).join(rel);
        if dev.exists() {
            return Some(dev);
        }
    }
    None
}

// ── Активная ASR-модель ──
pub fn active_id() -> String {
    crate::engine::store::get_setting("active_asr").unwrap_or_else(|| "gigaam".to_string())
}

pub fn set_active(id: &str) -> Result<(), String> {
    if find(id).map(|s| s.kind == "asr").unwrap_or(false) {
        crate::engine::store::set_setting("active_asr", id)
    } else {
        Err("not a language model".into())
    }
}

/// ASR-модели для селектора активной модели.
pub fn list_asr() -> Vec<ModelInfo> {
    list().into_iter().filter(|m| m.kind == "asr").collect()
}

/// Инфраструктурные модели (диаризация + инструменты), обязательные и ещё не установленные —
/// качать в фоне. Не-required инфра (напр. fallback-пунктуация) сюда не попадает.
pub fn missing_core() -> Vec<String> {
    CATALOG
        .iter()
        .filter(|s| s.kind != "asr" && s.required && !is_installed(s))
        .map(|s| s.id.to_string())
        .collect()
}

/// Русская модель БЕЗ встроенной пунктуации → нужен внешний RUPunct.
/// Сейчас в каталоге таких нет (GigaAM v3 пунктуирует сам). Хук для легаси/будущих моделей.
pub fn needs_ru_punct(id: &str) -> bool {
    matches!(id, "gigaam-v2")
}

/// Файлы активной ASR-модели (fallback на gigaam, если активная не установлена).
pub fn active_asr_files() -> Option<AsrFiles> {
    asr_files(&active_id()).or_else(|| asr_files("gigaam"))
}

// ── Модель диктовки (отдельная от общей ASR) ──

/// Активная модель диктовки. Пусто/не задано → следуем за активной ASR-моделью.
pub fn dict_asr_id() -> String {
    crate::engine::store::get_setting("dict_asr")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(active_id)
}

pub fn set_dict_asr(id: &str) -> Result<(), String> {
    // Пустая строка = «как активная ASR» (валидный выбор).
    if id.is_empty() || find(id).map(|s| s.kind == "asr").unwrap_or(false) {
        crate::engine::store::set_setting("dict_asr", id)
    } else {
        Err("not a language model".into())
    }
}

/// Файлы модели диктовки (fallback: активная ASR → gigaam).
pub fn dict_asr_files() -> Option<AsrFiles> {
    asr_files(&dict_asr_id()).or_else(active_asr_files)
}

fn asr_files(id: &str) -> Option<AsrFiles> {
    match id {
        "gigaam" => {
            let b = "sherpa-onnx-nemo-ctc-punct-giga-am-v3-russian-2025-12-16";
            Some(AsrFiles {
                engine: Engine::NemoCtc,
                model: resolve(&format!("{b}/model.int8.onnx"))?.to_str()?.into(),
                decoder: String::new(),
                joiner: String::new(),
                tokens: resolve(&format!("{b}/tokens.txt"))?.to_str()?.into(),
                language: String::new(),
            })
        }
        "parakeet" => {
            let b = "sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8";
            Some(AsrFiles {
                engine: Engine::Transducer,
                model: resolve(&format!("{b}/encoder.int8.onnx"))?.to_str()?.into(),
                decoder: resolve(&format!("{b}/decoder.int8.onnx"))?.to_str()?.into(),
                joiner: resolve(&format!("{b}/joiner.int8.onnx"))?.to_str()?.into(),
                tokens: resolve(&format!("{b}/tokens.txt"))?.to_str()?.into(),
                language: String::new(),
            })
        }
        "whisper-small" => {
            let b = "sherpa-onnx-whisper-small";
            Some(AsrFiles {
                engine: Engine::Whisper,
                model: resolve(&format!("{b}/small-encoder.int8.onnx"))?.to_str()?.into(),
                decoder: resolve(&format!("{b}/small-decoder.int8.onnx"))?.to_str()?.into(),
                joiner: String::new(),
                tokens: resolve(&format!("{b}/small-tokens.txt"))?.to_str()?.into(),
                language: "ru".into(),
            })
        }
        "whisper-turbo" => {
            let b = "sherpa-onnx-whisper-turbo";
            Some(AsrFiles {
                engine: Engine::Whisper,
                model: resolve(&format!("{b}/turbo-encoder.int8.onnx"))?.to_str()?.into(),
                decoder: resolve(&format!("{b}/turbo-decoder.int8.onnx"))?.to_str()?.into(),
                joiner: String::new(),
                tokens: resolve(&format!("{b}/turbo-tokens.txt"))?.to_str()?.into(),
                language: "ru".into(),
            })
        }
        _ => None,
    }
}

pub fn diarization() -> Option<(String, String)> {
    let seg = resolve("sherpa-onnx-pyannote-segmentation-3-0/model.onnx")?;
    // Предпочитаем новый мультиязычный эмбеддер; на старых установках (и в dev-фолбэке)
    // ещё может лежать прежний zh-cn — используем его, пока не докачается новый.
    let emb = resolve("3dspeaker_speech_campplus_sv_zh_en_16k-common_advanced.onnx")
        .or_else(|| resolve("3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx"))?;
    Some((seg.to_str()?.into(), emb.to_str()?.into()))
}

/// Путь к модели VAD (Silero) — для нарезки ASR по речи. None → слепые окна.
pub fn vad() -> Option<String> {
    resolve("silero_vad.onnx").and_then(|p| p.to_str().map(|s| s.to_string()))
}

/// Модель пунктуации (onnx + tokenizer.json). None → шаг пунктуации пропускается.
pub fn punct() -> Option<(String, String)> {
    let onnx = resolve("rupunct/rupunct_small_int8.onnx")?;
    let tok = resolve("rupunct/tokenizer.json")?;
    Some((onnx.to_str()?.into(), tok.to_str()?.into()))
}

pub fn ffmpeg() -> Option<String> {
    // каталог данных (установлено менеджером)
    let p = models_dir().join("bin").join(tool_exe("ffmpeg"));
    p.to_str().filter(|_| p.exists()).map(|s| s.to_string())
}

// ── LLM («Итоги встречи») ──

/// Активная LLM-модель. Дефолт — по железу: GPU или сильный CPU+RAM → качество,
/// слабые машины → быстрый тир (пользователь может сменить в Настройках).
pub fn active_llm_id() -> String {
    if let Some(v) = crate::engine::store::get_setting("active_llm") {
        return v;
    }
    let strong = crate::engine::gpu::should_offload().is_some() || {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let ram_gb = sys.total_memory() as f64 / 1e9;
        let cores = sys.physical_core_count().unwrap_or(4);
        ram_gb >= 12.0 && cores > 4
    };
    if strong { "llm-qwen3-4b" } else { "llm-qwen3-17b" }.to_string()
}

pub fn set_active_llm(id: &str) -> Result<(), String> {
    if find(id).map(|s| s.kind == "llm").unwrap_or(false) {
        crate::engine::store::set_setting("active_llm", id)
    } else {
        Err("not a summary model".into())
    }
}

/// LLM-модели для селектора «Модель для итогов».
pub fn list_llm() -> Vec<ModelInfo> {
    list().into_iter().filter(|m| m.kind == "llm").collect()
}

/// Путь к бинарнику llama-server (установлен менеджером).
pub fn llama_server() -> Option<String> {
    let p = models_dir().join("bin").join("llama").join(tool_exe("llama-server"));
    p.to_str().filter(|_| p.exists()).map(|s| s.to_string())
}

/// (llama-server, GGUF активной модели) — всё, что нужно для «Итогов».
/// None → фича недоступна, надо скачать (ensure_llm).
pub fn llm_files() -> Option<(String, String)> {
    let server = llama_server()?;
    let spec = find(&active_llm_id())?;
    let gguf = resolve(&spec.check_path())?;
    Some((server, gguf.to_str()?.to_string()))
}

/// У установленного llama нет Vulkan-бэкенда, а GPU на машине есть → перекачать
/// (существующие установки скачивали CPU-сборку; апгрейд — разовые ~32 МБ).
pub fn llama_needs_gpu_upgrade() -> bool {
    cfg!(windows)
        && llama_server().is_some()
        && crate::engine::gpu::should_offload().is_some()
        && !models_dir().join("bin").join("llama").join("ggml-vulkan.dll").exists()
}

/// Чего не хватает для «Итогов»: движок (или его GPU-апгрейд) и/или активная LLM-модель.
pub fn missing_llm() -> Vec<String> {
    let mut v = Vec::new();
    if llama_server().is_none() || llama_needs_gpu_upgrade() {
        v.push("llama".to_string());
    }
    let id = active_llm_id();
    if find(&id).map(|s| !is_installed(s)).unwrap_or(true) {
        v.push(id);
    }
    v
}
