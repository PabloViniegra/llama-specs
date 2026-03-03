pub mod compat;
pub mod database;
pub mod ollama;

use serde::{Deserialize, Serialize};

/// A model entry from the embedded database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    /// Ollama model name (e.g. "llama3:8b").
    pub name: String,
    /// Parameter count in billions (e.g. 8.0 for an 8B model).
    pub params_b: f64,
    /// Model family (e.g. "Llama 3").
    pub family: String,
    /// Short human-readable description.
    pub description: String,
}

/// The compatibility verdict for a model given the current hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Model fits entirely in free VRAM — fast GPU inference.
    Optimal,
    /// Model needs VRAM + RAM — partial GPU offloading, slower.
    Hybrid,
    /// Model only fits in RAM — CPU execution only.
    Slow,
    /// Model exceeds total RAM + VRAM — cannot run.
    Incompatible,
}

/// Detailed result of the compatibility calculation.
#[derive(Debug, Clone)]
pub struct CompatResult {
    pub verdict: Verdict,
    /// Total estimated memory the model needs (MB).
    pub estimated_mb: u64,
    /// How much of that would be loaded onto VRAM (MB).
    pub vram_used_mb: u64,
    /// How much of that would spill into RAM (MB).
    pub ram_used_mb: u64,
}
