//! Сколько потоков/воркеров давать ONNX-моделям на этой машине.
//!
//! Два режима:
//! - один прогон на всех ядрах (диктовка, диаризация, Whisper) — `ort_threads()`;
//! - много независимых прогонов по 1 потоку (файловый ASR) — `asr_workers()`.
//!
//! Логические (SMT) ядра не считаем: ORT на них не ускоряется, а на барьерах
//! синхронизации «близнецы» тормозят друг друга.

use std::sync::OnceLock;

/// Физические ядра (на гибридных CPU — все, P+E).
pub fn physical_cores() -> usize {
    static N: OnceLock<usize> = OnceLock::new();
    *N.get_or_init(|| {
        sysinfo::System::new()
            .physical_core_count()
            .filter(|&n| n > 0)
            .unwrap_or_else(logical_cores)
    })
}

pub fn logical_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Потоки ORT для ОДНОГО прогона. На Apple Silicon — только P-ядра: E-ядра в
/// intra-op параллелизме заставляют быстрые ядра ждать медленные на каждом барьере.
pub fn ort_threads() -> i32 {
    #[cfg(target_os = "macos")]
    if let Some(p) = macos_perf_cores() {
        return p.clamp(1, 16) as i32;
    }
    physical_cores().clamp(1, 16) as i32
}

/// Параллельные декодеры файлового ASR (по 1 потоку ORT каждый). Сегменты раздаются
/// из общей очереди, поэтому E-ядра здесь не мешают — просто берут меньше работы.
pub fn asr_workers() -> usize {
    physical_cores().clamp(1, 16)
}

#[cfg(target_os = "macos")]
fn macos_perf_cores() -> Option<usize> {
    let mut v: libc::c_int = 0;
    let mut size = std::mem::size_of::<libc::c_int>();
    let r = unsafe {
        libc::sysctlbyname(
            c"hw.perflevel0.physicalcpu".as_ptr(),
            &mut v as *mut libc::c_int as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (r == 0 && v > 0).then_some(v as usize)
}
