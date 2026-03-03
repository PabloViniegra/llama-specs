use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};
use rust_i18n::t;

use crate::app::{CatalogStatus, OllamaStatus};

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
                    .fg(ratatui::style::Color::Green)
                    .add_modifier(Modifier::DIM),
            ),
            CatalogStatus::Failed => "  ⚠ Offline catalog".dim().red(),
        };

        let title_line = Line::from(vec![
            Span::styled(
                format!(" {} ", t!("app_title")),
                Style::default()
                    .fg(ratatui::style::Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            "— ".dim(),
            t!("app_subtitle").dim(),
            format!("  [{}]", self.lang.to_uppercase()).dim(),
        ]);

        let status_line = Line::from(vec![" ".into(), ollama_span, catalog_span]);

        let block = Block::bordered()
            .title(title_line)
            .title_bottom(status_line);

        Paragraph::new("").block(block).render(area, buf);
    }
}
