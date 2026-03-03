use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};
use rust_i18n::t;

use crate::{
    app::AppMode,
    models::{ModelEntry, Verdict},
};

pub struct ModelList<'a> {
    pub models: &'a [ModelEntry],
    /// Indices into `models` that are currently visible (filtered).
    pub visible: &'a [usize],
    pub installed: &'a std::collections::HashSet<String>,
    pub search_query: &'a str,
    pub mode: AppMode,
    /// Verdict per entry (parallel to `models`), used for colour-coding.
    pub verdicts: &'a [Option<crate::models::Verdict>],
}

impl StatefulWidget for ModelList<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // If in search mode, split off a one-line search box at the bottom.
        let (list_area, search_area) = if self.mode == AppMode::Search {
            let [l, s] = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(area);
            (l, Some(s))
        } else {
            (area, None)
        };

        // Build list items
        let items: Vec<ListItem> = self
            .visible
            .iter()
            .map(|&idx| {
                let model = &self.models[idx];
                let installed = self.installed.contains(&model.name);
                let verdict = self.verdicts.get(idx).and_then(|v| *v);
                build_item(model, installed, verdict)
            })
            .collect();

        let empty_hint = if self.visible.is_empty() {
            t!("list_empty").to_string()
        } else {
            String::new()
        };

        let title = Span::styled(
            format!(" {} ", t!("list_title")),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );

        let list = List::new(items)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(title),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        if self.visible.is_empty() {
            Paragraph::new(empty_hint)
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(Span::styled(
                            format!(" {} ", t!("list_title")),
                            Style::default().fg(Color::Cyan),
                        )),
                )
                .dark_gray()
                .render(list_area, buf);
        } else {
            StatefulWidget::render(list, list_area, buf, state);
        }

        // Search box
        if let Some(s_area) = search_area {
            let query_display = format!("/ {}_", self.search_query);
            let search_block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(Span::styled(" Search ", Style::default().fg(Color::Yellow)));
            Paragraph::new(query_display)
                .block(search_block)
                .yellow()
                .render(s_area, buf);
        }
    }
}

fn build_item<'a>(
    model: &'a ModelEntry,
    installed: bool,
    verdict: Option<Verdict>,
) -> ListItem<'a> {
    // ── Installed indicator ────────────────────────────────────────────────
    let (install_sym, install_style) = if installed {
        ("✓ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        ("  ", Style::default())
    };

    // ── Verdict colour dot ─────────────────────────────────────────────────
    let (verdict_bullet, verdict_color) = match verdict {
        Some(Verdict::Optimal)      => ("●", Color::Green),
        Some(Verdict::Hybrid)       => ("●", Color::Yellow),
        Some(Verdict::Slow)         => ("●", Color::Rgb(255, 140, 0)),
        Some(Verdict::Incompatible) => ("●", Color::Red),
        None                        => ("○", Color::DarkGray),
    };

    // ── Model name ─────────────────────────────────────────────────────────
    let name_style = if installed {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    // ── Parameter count ────────────────────────────────────────────────────
    let params_str = if model.params_b <= 0.0 {
        " ?".to_owned()
    } else if model.params_b < 1.0 {
        format!(" {:.1}B", model.params_b)
    } else {
        format!(" {:.0}B", model.params_b)
    };

    let line = Line::from(vec![
        Span::styled(install_sym, install_style),
        Span::styled(verdict_bullet, Style::default().fg(verdict_color)),
        " ".into(),
        Span::styled(model.name.clone(), name_style),
        Span::styled(
            format!("{}  {}", params_str, model.family),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    ListItem::new(line)
}
