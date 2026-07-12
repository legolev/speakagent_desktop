//! Аудио-утилиты ядра. Фаза 1 — только базовая инфа о файле.
//! Далее: декод через bundled ffmpeg → 16kHz mono PCM (порт worker/audio.py).

use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct FileInfo {
    pub name: String,
    pub size_bytes: u64,
    pub size_human: String,
}

pub fn file_info(path: &str) -> Result<FileInfo, String> {
    let p = Path::new(path);
    let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let name = p
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let size = meta.len();
    Ok(FileInfo {
        name,
        size_bytes: size,
        size_human: human_size(size),
    })
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut b = bytes as f64;
    let mut i = 0;
    while b >= 1024.0 && i < UNITS.len() - 1 {
        b /= 1024.0;
        i += 1;
    }
    format!("{:.1} {}", b, UNITS[i])
}
