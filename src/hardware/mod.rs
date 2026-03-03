pub mod gpu;
pub mod ram;

/// Opaque handle to the NVML context (NVIDIA Management Library).
/// On macOS, NVML does not exist so this resolves to `Option<()>`.
#[cfg(not(target_os = "macos"))]
pub type NvmlHandle = Option<nvml_wrapper::Nvml>;

#[cfg(target_os = "macos")]
pub type NvmlHandle = Option<()>;

/// System architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Arm,
    Other,
}

impl Arch {
    pub fn detect() -> Self {
        match std::env::consts::ARCH {
            "x86_64" => Self::X86_64,
            "aarch64" | "arm" => Self::Arm,
            _ => Self::Other,
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X86_64 => write!(f, "x86_64"),
            Self::Arm => write!(f, "ARM"),
            Self::Other => write!(f, "Unknown"),
        }
    }
}

/// NVIDIA GPU information queried via NVML.
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub vram_total_mb: u64,
    pub vram_free_mb: u64,
}

/// Snapshot of the system's hardware state.
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub ram_total_mb: u64,
    pub ram_available_mb: u64,
    pub gpu: Option<GpuInfo>,
    pub arch: Arch,
}

impl HardwareInfo {
    /// Performs the initial hardware detection.
    /// GPU errors are silently ignored (no NVIDIA GPU or drivers not installed).
    pub fn detect(sys: &mut sysinfo::System) -> Self {
        let (ram_total_mb, ram_available_mb) = ram::query(sys);
        let gpu = gpu::query();
        let arch = Arch::detect();
        Self {
            ram_total_mb,
            ram_available_mb,
            gpu,
            arch,
        }
    }

    /// Refreshes RAM from an already-initialized `sysinfo::System`.
    pub fn refresh_ram(&mut self, sys: &mut sysinfo::System) {
        let (total, avail) = ram::query(sys);
        self.ram_total_mb = total;
        self.ram_available_mb = avail;
    }

    /// Refreshes GPU VRAM (if a GPU handle is available).
    pub fn refresh_gpu(&mut self, nvml: &NvmlHandle) {
        self.gpu = gpu::query_with_handle(nvml);
    }
}
