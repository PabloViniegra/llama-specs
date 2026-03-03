# LlamaSpec

A terminal user interface (TUI) for checking Ollama model compatibility with your hardware before downloading. Written in Rust.

LlamaSpec eliminates the trial-and-error of loading large language models by scanning your system resources — RAM, VRAM, and CPU — and giving you an instant verdict on whether a given model will run optimally, slowly, or not at all on your machine.

---

## Features

- **Hardware detection:** Reads total and available RAM, GPU VRAM (NVIDIA via NVML), and CPU architecture at startup.
- **Compatibility engine:** Estimates memory requirements using quantization math and classifies each model into one of four verdicts.
- **Interactive model browser:** Fuzzy search across a curated database of popular Ollama models (Llama 3, Mistral, Gemma, Phi-3, Command-R, and more).
- **Real-time resource monitor:** Sidebar progress bars show current usage vs. estimated usage for the selected model.
- **Ollama integration:** Detects models already installed locally via the Ollama API (`localhost:11434`) and marks them with `[v]`.
- **Localization:** UI available in English and Spanish (`rust-i18n`).

---

## Compatibility Verdicts

The engine evaluates each model using the following memory estimate:

```
Memory (GB) = (Parameters × Bits / 8) × 1.25
```

Default quantization assumed: **Q4_K_M (4 bits)**.

| Verdict     | Condition                              |
|-------------|----------------------------------------|
| Optimal     | Model fits entirely in VRAM            |
| Hybrid      | Model fits in VRAM + RAM combined      |
| Slow        | Model fits only in RAM (CPU execution) |
| Incompatible| Model exceeds total RAM + VRAM         |

---

## UI Layout

```
+--- Header: App name + Ollama service status (localhost:11434) ---+
| Sidebar (left)  | Main (center)          | Footer (bottom)       |
| RAM/VRAM bars   | Filterable model list  | Verdict + min specs   |
+-----------------+------------------------+-----------------------+
```

**Keybindings:**

| Key        | Action                    |
|------------|---------------------------|
| `/`        | Enter search/filter mode  |
| `Esc`      | Exit search mode          |
| Up / Down  | Navigate model list       |
| `q`        | Quit                      |

---

## Requirements

- **Rust** 1.85+ (edition 2024)
- **Ollama** running on `localhost:11434` (optional — app works without it)
- **NVIDIA GPU** with drivers installed for VRAM detection (optional)

---

## Installation

Clone the repository and build in release mode:

```bash
git clone https://github.com/your-username/llama-specs.git
cd llama-specs
cargo build --release
./target/release/llama-specs
```

Or run directly during development:

```bash
cargo run
```

---

## Development

```bash
# Check for compilation errors
cargo check

# Run tests
cargo test

# Run a specific test
cargo test <test_name>

# Lint
cargo clippy

# Format
cargo fmt
```

---

## Tech Stack

| Crate           | Purpose                            |
|-----------------|------------------------------------|
| `ratatui`       | TUI framework                      |
| `crossterm`     | Terminal backend and event handling|
| `tokio`         | Async runtime                      |
| `sysinfo`       | RAM and CPU metrics                |
| `nvml-wrapper`  | NVIDIA GPU / VRAM telemetry        |
| `fuzzy-matcher` | Model fuzzy search                 |
| `serde_json`    | Static model database              |
| `reqwest`       | Ollama API HTTP client             |
| `rust-i18n`     | English / Spanish localization     |
| `color-eyre`    | Error reporting                    |

---

## Roadmap

- [x] Milestone 1: Hardware detection (RAM / CPU) and basic Ratatui layout
- [x] Milestone 2: Memory calculation logic and static model database
- [ ] Milestone 3: NVIDIA GPU / VRAM support and fuzzy model search
- [ ] Milestone 4: Ollama API integration for installed model detection

---

## License

MIT
