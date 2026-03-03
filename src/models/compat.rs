use crate::hardware::HardwareInfo;

use super::{CompatResult, ModelEntry, Verdict};

/// Bits per parameter for Q4_K_M quantization.
const BITS_PER_PARAM: f64 = 4.0;

/// Overhead factor that accounts for KV cache and activations (25 %).
const OVERHEAD: f64 = 1.25;

const BYTES_PER_MB: f64 = 1_048_576.0;

/// Estimates how much memory a model needs in MB.
///
/// Formula: `(params × bits / 8) × overhead` converted to MB.
pub fn estimate_mb(model: &ModelEntry) -> u64 {
    let params = model.params_b * 1_000_000_000.0;
    let bytes = (params * BITS_PER_PARAM / 8.0) * OVERHEAD;
    (bytes / BYTES_PER_MB) as u64
}

/// Returns `true` if this model's parameter count is known.
/// Models fetched from the online catalog may have `params_b = 0.0` when the
/// parameter count cannot be determined; they skip verdict computation.
pub fn params_known(model: &ModelEntry) -> bool {
    model.params_b > 0.0
}

/// Evaluates whether a model is compatible with the given hardware.
/// Returns `None` when the parameter count is unknown (params_b == 0.0).
pub fn evaluate(model: &ModelEntry, hw: &HardwareInfo) -> Option<CompatResult> {
    if !params_known(model) {
        return None;
    }
    Some(evaluate_inner(model, hw))
}

fn evaluate_inner(model: &ModelEntry, hw: &HardwareInfo) -> CompatResult {
    let estimated_mb = estimate_mb(model);

    let vram_free = hw.gpu.as_ref().map_or(0, |g| g.vram_free_mb);
    let ram_avail = hw.ram_available_mb;

    if estimated_mb <= vram_free {
        // Entire model fits in free VRAM.
        CompatResult {
            verdict: Verdict::Optimal,
            estimated_mb,
            vram_used_mb: estimated_mb,
            ram_used_mb: 0,
        }
    } else if hw.gpu.is_some() && estimated_mb <= vram_free + ram_avail {
        // Partial GPU offload: as much as possible into VRAM, the rest in RAM.
        let vram_used = vram_free;
        let ram_used = estimated_mb.saturating_sub(vram_used);
        CompatResult {
            verdict: Verdict::Hybrid,
            estimated_mb,
            vram_used_mb: vram_used,
            ram_used_mb: ram_used,
        }
    } else if estimated_mb <= ram_avail {
        // No GPU or VRAM too small — CPU-only execution.
        CompatResult {
            verdict: Verdict::Slow,
            estimated_mb,
            vram_used_mb: 0,
            ram_used_mb: estimated_mb,
        }
    } else {
        // Cannot fit in any available memory.
        CompatResult {
            verdict: Verdict::Incompatible,
            estimated_mb,
            vram_used_mb: 0,
            ram_used_mb: 0,
        }
    }
}
