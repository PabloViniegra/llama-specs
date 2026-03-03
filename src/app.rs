use std::collections::HashSet;
use std::time::Instant;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::{Terminal, backend::Backend, widgets::ListState};
use tokio::time::{Duration, interval};

use crate::{
    hardware::{self, HardwareInfo},
    models::{self, ModelEntry, Verdict},
    ui,
};

/// Whether the user is in normal navigation or fuzzy-search mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Search,
}

/// Current connection state of the Ollama daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OllamaStatus {
    Checking,
    Running,
    Stopped,
}

/// Download state of the online model catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogStatus {
    /// Initial fetch in progress.
    Loading,
    /// Catalog loaded successfully.
    Ready,
    /// Network or parse failure; showing offline fallback.
    Failed,
}

/// Messages that drive state updates (Elm-style).
pub enum Msg {
    /// Move list selection up.
    Up,
    /// Move list selection down.
    Down,
    /// Enter fuzzy-search mode.
    EnterSearch,
    /// Exit search mode, keeping the current query.
    ExitSearch,
    /// Append a character to the search query.
    SearchChar(char),
    /// Delete last character from the search query.
    SearchBackspace,
    /// Clear search and exit search mode.
    ClearSearch,
    /// Toggle between EN and ES.
    ToggleLang,
    /// Refresh hardware metrics.
    HardwareTick,
    /// Ollama status received from background task.
    OllamaUpdate {
        running: bool,
        installed: Vec<String>,
    },
    /// Full online catalog fetched successfully.
    CatalogLoaded(Vec<ModelEntry>),
    /// Online catalog fetch failed; keep showing the offline fallback.
    CatalogFailed,
    /// Copy "ollama run <selected>" to clipboard.
    CopyCommand,
    /// Quit the application.
    Quit,
}

/// Full application state.
pub struct AppState {
    pub hardware: HardwareInfo,
    /// All models currently visible in the list (from DB or online catalog).
    pub models: Vec<ModelEntry>,
    /// Indices into `models` that survive the current search filter.
    pub visible: Vec<usize>,
    /// Model names installed locally (from Ollama API).
    pub installed: HashSet<String>,
    /// Pre-computed verdicts for each model (parallel to `models`).
    pub verdicts: Vec<Option<Verdict>>,
    pub ollama_status: OllamaStatus,
    pub catalog_status: CatalogStatus,
    pub mode: AppMode,
    /// Index into `visible` (not into `models`).
    pub selected_idx: usize,
    pub list_state: ListState,
    pub search_query: String,
    pub should_quit: bool,
    /// Temporary footer notification after a clipboard copy (message + timestamp).
    pub clipboard_msg: Option<(String, Instant)>,
    sys: sysinfo::System,
    nvml: hardware::NvmlHandle,
    matcher: SkimMatcherV2,
}

impl AppState {
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new();
        let nvml = hardware::gpu::init_nvml();
        let hardware = HardwareInfo::detect(&mut sys);
        let models = models::database::load();
        let n = models.len();
        let visible: Vec<usize> = (0..n).collect();
        let verdicts: Vec<Option<Verdict>> = vec![None; n];

        let mut list_state = ListState::default();
        if !visible.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            hardware,
            models,
            visible,
            installed: HashSet::new(),
            verdicts,
            ollama_status: OllamaStatus::Checking,
            catalog_status: CatalogStatus::Loading,
            mode: AppMode::Normal,
            selected_idx: 0,
            list_state,
            search_query: String::new(),
            should_quit: false,
            clipboard_msg: None,
            sys,
            nvml,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Applies a `Msg` to the state.
    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Quit => self.should_quit = true,

            Msg::Up => self.move_selection(-1),
            Msg::Down => self.move_selection(1),

            Msg::EnterSearch => {
                self.mode = AppMode::Search;
            }
            Msg::ExitSearch => {
                self.mode = AppMode::Normal;
            }
            Msg::ClearSearch => {
                self.search_query.clear();
                self.mode = AppMode::Normal;
                self.refilter();
            }
            Msg::SearchChar(c) => {
                self.search_query.push(c);
                self.refilter();
            }
            Msg::SearchBackspace => {
                self.search_query.pop();
                self.refilter();
            }

            Msg::ToggleLang => {
                let next = if rust_i18n::locale().to_string() == "en" {
                    "es"
                } else {
                    "en"
                };
                rust_i18n::set_locale(next);
            }

            Msg::CopyCommand => {
                if let Some(&idx) = self.visible.get(self.selected_idx) {
                    let model_name = self.models[idx].name.clone();
                    let command = format!("ollama run {model_name}");
                    let msg = match arboard::Clipboard::new()
                        .and_then(|mut cb| cb.set_text(&command))
                    {
                        Ok(()) => format!(
                            "{}  {}",
                            rust_i18n::t!("clipboard_copied"),
                            command
                        ),
                        Err(_) => rust_i18n::t!("clipboard_error").to_string(),
                    };
                    self.clipboard_msg = Some((msg, Instant::now()));
                }
            }

            Msg::HardwareTick => {
                self.hardware.refresh_ram(&mut self.sys);
                self.hardware.refresh_gpu(&self.nvml);
                self.recompute_verdicts();
                // Expire clipboard notification after 3 seconds.
                if let Some((_, at)) = &self.clipboard_msg
                    && at.elapsed() > std::time::Duration::from_secs(3)
                {
                    self.clipboard_msg = None;
                }
            }

            Msg::OllamaUpdate { running, installed } => {
                self.ollama_status = if running {
                    OllamaStatus::Running
                } else {
                    OllamaStatus::Stopped
                };
                self.installed = installed.into_iter().collect();
            }

            Msg::CatalogLoaded(new_models) => {
                self.catalog_status = CatalogStatus::Ready;
                // Replace model list, preserving the current filter query.
                self.models = new_models;
                self.verdicts = vec![None; self.models.len()];
                self.refilter();
            }

            Msg::CatalogFailed => {
                self.catalog_status = CatalogStatus::Failed;
                // Keep showing the offline fallback (already in self.models).
            }
        }
    }

    fn move_selection(&mut self, delta: i64) {
        if self.visible.is_empty() {
            return;
        }
        let len = self.visible.len() as i64;
        let next = (self.selected_idx as i64 + delta).rem_euclid(len) as usize;
        self.selected_idx = next;
        self.list_state.select(Some(next));
    }

    /// Re-applies fuzzy filtering whenever the query or model list changes.
    fn refilter(&mut self) {
        if self.search_query.is_empty() {
            self.visible = (0..self.models.len()).collect();
        } else {
            let query = self.search_query.clone();
            let mut scored: Vec<(i64, usize)> = self
                .models
                .iter()
                .enumerate()
                .filter_map(|(i, m)| {
                    // Match against name + family + description for better recall.
                    let haystack = format!("{} {} {}", m.name, m.family, m.description);
                    self.matcher
                        .fuzzy_match(&haystack, &query)
                        .map(|score| (score, i))
                })
                .collect();
            // Best matches first.
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            self.visible = scored.into_iter().map(|(_, i)| i).collect();
        }

        // Clamp selected_idx to new visible length.
        let new_len = self.visible.len();
        if new_len == 0 {
            self.selected_idx = 0;
            self.list_state.select(None);
        } else {
            self.selected_idx = self.selected_idx.min(new_len - 1);
            self.list_state.select(Some(self.selected_idx));
        }

        self.recompute_verdicts();
    }

    /// Pre-computes compatibility verdicts for every model given current hardware.
    fn recompute_verdicts(&mut self) {
        let hw = &self.hardware;
        for (i, model) in self.models.iter().enumerate() {
            self.verdicts[i] = crate::models::compat::evaluate(model, hw).map(|r| r.verdict);
        }
    }
}

/// Entry point for the async TUI event loop.
pub async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    mut state: AppState,
) -> color_eyre::Result<()>
where
    B::Error: Send + Sync + 'static,
{
    // Compute initial verdicts with the offline database.
    state.recompute_verdicts();

    // Shared mpsc channel for all background messages.
    let (bg_tx, mut bg_rx) = tokio::sync::mpsc::channel::<Msg>(16);

    // Background task A: Ollama daemon poller (every 5 s).
    let ollama_tx = bg_tx.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let running = crate::models::ollama::is_running(&client).await;
            let installed = if running {
                crate::models::ollama::fetch_installed(&client).await
            } else {
                Vec::new()
            };
            let _ = ollama_tx
                .send(Msg::OllamaUpdate { running, installed })
                .await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Background task B: online catalog fetch (one-shot at startup).
    let catalog_tx = bg_tx.clone();
    let static_db = state.models.clone(); // snapshot of the offline fallback
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        match crate::models::ollama::fetch_library(&client, &static_db).await {
            Ok(models) if !models.is_empty() => {
                let _ = catalog_tx.send(Msg::CatalogLoaded(models)).await;
            }
            Ok(_) | Err(_) => {
                let _ = catalog_tx.send(Msg::CatalogFailed).await;
            }
        }
    });

    let mut events = EventStream::new();
    let mut hw_tick = interval(Duration::from_secs(1));

    loop {
        terminal.draw(|f| ui::view(&mut state, f))?;

        tokio::select! {
            maybe_event = events.next() => {
                let Some(Ok(event)) = maybe_event else { continue };
                if let Some(msg) = event_to_msg(&event, state.mode) {
                    state.update(msg);
                }
            }
            _ = hw_tick.tick() => {
                state.update(Msg::HardwareTick);
            }
            Some(msg) = bg_rx.recv() => {
                state.update(msg);
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

/// Translates a crossterm event into an application message.
fn event_to_msg(event: &Event, mode: AppMode) -> Option<Msg> {
    let Event::Key(key) = event else { return None };

    // Only process key-press events (ignore repeat/release on Windows).
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match mode {
        AppMode::Normal => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => Some(Msg::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Msg::CopyCommand)
            }
            KeyCode::Char('/') => Some(Msg::EnterSearch),
            KeyCode::Tab => Some(Msg::ToggleLang),
            KeyCode::Up | KeyCode::Char('k') => Some(Msg::Up),
            KeyCode::Down | KeyCode::Char('j') => Some(Msg::Down),
            _ => None,
        },
        AppMode::Search => match key.code {
            KeyCode::Esc => Some(Msg::ClearSearch),
            KeyCode::Enter => Some(Msg::ExitSearch),
            KeyCode::Backspace => Some(Msg::SearchBackspace),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Msg::CopyCommand)
            }
            KeyCode::Up => Some(Msg::Up),
            KeyCode::Down => Some(Msg::Down),
            KeyCode::Char(c) => Some(Msg::SearchChar(c)),
            _ => None,
        },
    }
}
