use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Widget},
};
use rust_i18n::t;

use crate::app::{CatalogStatus, OllamaStatus};

const GITHUB_URL: &str = "https://github.com/PabloViniegra/llama-specs";
// Nerd Font GitHub icon (U+E709) + repo short name shown in the header title.
const GITHUB_LABEL: &str = " \u{e709} llama-specs ";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Header<'a> {
    pub ollama_status: &'a OllamaStatus,
    pub catalog_status: &'a CatalogStatus,
    pub model_count: usize,
    pub lang: &'a str,
}

impl Widget for Header<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let ollama_span: Span = match self.ollama_status {
            OllamaStatus::Running => t!("ollama_running").bold().green(),
            OllamaStatus::Stopped => t!("ollama_stopped").bold().red(),
            OllamaStatus::Checking => t!("ollama_checking").dim().yellow(),
        };

        let catalog_span: Span = match self.catalog_status {
            CatalogStatus::Loading => "  🔄 Loading catalog...".dim().yellow(),
            CatalogStatus::Ready => Span::styled(
                format!("  ✓ {} models", self.model_count),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::DIM),
            ),
            CatalogStatus::Failed => "  ⚠ Offline catalog".dim().red(),
        };

        // Left title: app icon + name + version + lang
        let left_title = Line::from(vec![
            Span::styled(
                format!(" 🦙 {} v{} ", t!("app_title"), APP_VERSION),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}] ", self.lang.to_uppercase()),
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        // Right title: GitHub Nerd Font icon + repo name (OSC 8 hyperlink injected below).
        let github_title = Line::from(vec![Span::styled(
            GITHUB_LABEL,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::UNDERLINED),
        )]);

        // Bottom status line
        let status_line = Line::from(vec![" ".into(), ollama_span, catalog_span]);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(left_title)
            .title_alignment(Alignment::Left)
            .title_top(github_title.alignment(Alignment::Right))
            .title_bottom(status_line);

        Paragraph::new("").block(block).render(area, buf);

        // Inject OSC 8 hyperlink into the buffer cells that hold the GitHub label on
        // the top border row. Terminals supporting OSC 8 (Windows Terminal, iTerm2,
        // kitty, foot) will render the label as a clickable hyperlink.
        render_osc8_hyperlink(buf, area, GITHUB_URL, GITHUB_LABEL);
    }
}

/// Writes OSC 8 hyperlink escape sequences around the cells that render
/// `label` on the top border row of `area`. The label is right-aligned inside
/// the block border (one char inward from the right `╮`).
///
/// Terminals that support OSC 8 (Windows Terminal, iTerm2, kitty, foot …)
/// will render those cells as a clickable hyperlink pointing to `url`.
fn render_osc8_hyperlink(buf: &mut Buffer, area: Rect, url: &str, label: &str) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let border_row = area.y;
    // Each Unicode scalar value occupies one buffer cell in ratatui (wide chars
    // like the Nerd Font icon get a placeholder cell next to them, so char count
    // ≈ column count for our purposes here).
    let label_cols = label.chars().count() as u16;

    // Right-aligned: the label ends one cell before the right border char `╮`.
    // layout: … ─ ─ [label] ╮
    //                        ^ area.x + area.width - 1
    //              ^─────────^ label_cols + 1 (border)
    let label_start = (area.x + area.width).saturating_sub(label_cols + 1);

    let osc8_open = format!("\x1b]8;;{url}\x1b\\");
    let osc8_close = "\x1b]8;;\x1b\\";

    // Prepend the opening sequence to the first cell of the label.
    if let Some(cell) = buf.cell_mut((label_start, border_row)) {
        let sym = cell.symbol().to_owned();
        cell.set_symbol(&format!("{osc8_open}{sym}"));
    }

    // Append the closing sequence to the last cell of the label.
    let label_end = label_start + label_cols - 1;
    if label_end < area.x + area.width {
        if let Some(cell) = buf.cell_mut((label_end, border_row)) {
            let sym = cell.symbol().to_owned();
            cell.set_symbol(&format!("{sym}{osc8_close}"));
        }
    }
}
