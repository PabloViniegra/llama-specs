use super::GpuInfo;

#[cfg(not(target_os = "macos"))]
use nvml_wrapper::Nvml;

#[cfg(not(target_os = "macos"))]
const BYTES_PER_MB: u64 = 1_048_576;

/// Initializes NVML and returns a handle, silently returning `None` on any error.
/// Should be called once at startup; the handle is reused for subsequent refreshes.
#[cfg(not(target_os = "macos"))]
pub fn init_nvml() -> Option<Nvml> {
    Nvml::init().ok()
}

#[cfg(target_os = "macos")]
pub fn init_nvml() -> Option<()> {
    None
}

/// Queries NVIDIA GPU info from an existing NVML handle (first device only).
#[cfg(not(target_os = "macos"))]
pub fn query_with_handle(nvml: &Option<Nvml>) -> Option<GpuInfo> {
    let nvml = nvml.as_ref()?;
    let device = nvml.device_by_index(0).ok()?;
    let name = device.name().ok()?;
    let mem = device.memory_info().ok()?;
    Some(GpuInfo {
        name,
        vram_total_mb: mem.total / BYTES_PER_MB,
        vram_free_mb: mem.free / BYTES_PER_MB,
    })
}

#[cfg(target_os = "macos")]
pub fn query_with_handle(_nvml: &Option<()>) -> Option<GpuInfo> {
    None
}

/// One-shot GPU query without a persistent handle (used for initial detection).
#[cfg(not(target_os = "macos"))]
pub fn query() -> Option<GpuInfo> {
    let nvml = Nvml::init().ok()?;
    query_with_handle(&Some(nvml))
}

#[cfg(target_os = "macos")]
pub fn query() -> Option<GpuInfo> {
    None
}
