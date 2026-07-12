//! Детект GPU — решаем, ускорять ли «Итоги» видеокартой (Vulkan-бэкенд llama-server).
//! Windows: DXGI (DedicatedVideoMemory 64-битный — CIM/wmic врут после 4 ГБ).
//! ASR это не касается: int8-модели sherpa на GPU не быстрее (замерено в Фазе 0).

pub struct GpuInfo {
    pub name: String,
    pub vram_bytes: u64,
}

impl GpuInfo {
    pub fn vram_gb(&self) -> f64 {
        self.vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

/// Минимум видеопамяти для выгрузки 4B-модели с контекстом (и отсечка iGPU,
/// которые на Vulkan бывают МЕДЛЕННЕЕ CPU). Используется только в Windows-ветке
/// (на macOS Metal всегда есть — порог по unified RAM внутри should_offload).
#[cfg(windows)]
const MIN_VRAM_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Все аппаратные адаптеры (без Microsoft Basic Render Driver).
#[cfg(windows)]
pub fn detect_gpus() -> Vec<GpuInfo> {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
    };
    let mut out = Vec::new();
    unsafe {
        let factory: IDXGIFactory1 = match CreateDXGIFactory1() {
            Ok(f) => f,
            Err(_) => return out,
        };
        let mut i = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            i += 1;
            let Ok(desc) = adapter.GetDesc1() else { continue };
            if desc.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
                continue;
            }
            let name = String::from_utf16_lossy(&desc.Description)
                .trim_end_matches('\0')
                .to_string();
            out.push(GpuInfo {
                name,
                vram_bytes: desc.DedicatedVideoMemory as u64,
            });
        }
    }
    out
}

#[cfg(not(windows))]
pub fn detect_gpus() -> Vec<GpuInfo> {
    Vec::new()
}

/// Есть ли Vulkan-загрузчик (без него GPU-бэкенд просто не поднимется).
#[cfg(windows)]
pub fn vulkan_available() -> bool {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    std::path::Path::new(&root)
        .join("System32")
        .join("vulkan-1.dll")
        .exists()
}

#[cfg(not(windows))]
pub fn vulkan_available() -> bool {
    false
}

/// Лучший адаптер (для отображения в системной строке — без порога).
pub fn best_gpu() -> Option<GpuInfo> {
    // На macOS дискретных адаптеров нет (detect_gpus пуст) → показываем ускоритель Metal,
    // иначе в UI «Итоги ускоряются видеокартой ()» с пустыми скобками.
    #[cfg(target_os = "macos")]
    {
        return should_offload();
    }
    #[cfg(not(target_os = "macos"))]
    detect_gpus().into_iter().max_by_key(|g| g.vram_bytes)
}

/// GPU, на который стоит выгружать LLM. None → считаем на CPU (`--device none`).
pub fn should_offload() -> Option<GpuInfo> {
    #[cfg(windows)]
    {
        if !vulkan_available() {
            return None;
        }
        detect_gpus()
            .into_iter()
            .filter(|g| g.vram_bytes >= MIN_VRAM_BYTES)
            .max_by_key(|g| g.vram_bytes)
    }
    #[cfg(target_os = "macos")]
    {
        // Apple Silicon: Metal всегда есть; unified memory ≈ 75% RAM доступно GPU
        #[cfg(target_arch = "aarch64")]
        {
            let mut sys = sysinfo::System::new();
            sys.refresh_memory();
            return Some(GpuInfo {
                name: "Apple Silicon".into(),
                vram_bytes: (sys.total_memory() as f64 * 0.75) as u64,
            });
        }
        #[cfg(not(target_arch = "aarch64"))]
        return None;
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        None
    }
}
