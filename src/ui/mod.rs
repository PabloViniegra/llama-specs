pub mod header;
pub mod model_list;
pub mod sidebar;

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};
use rust_i18n::t;

use crate::app::{AppMode, AppState};

use self::{header::Header, model_list::ModelList, sidebar::Sidebar};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Renders the full TUI. Called once per frame from the main loop.
pub fn view(state: &mut AppState, frame: &mut Frame) {
    let [header_area, main_area, footer_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [sidebar_area, list_area] =
        Layout::horizontal([Constraint::Length(36), Constraint::Fill(1)]).areas(main_area);

    // Header
    frame.render_widget(
        Header {
            ollama_status: &state.ollama_status,
            catalog_status: &state.catalog_status,
            model_count: state.models.len(),
            lang: rust_i18n::locale().as_ref(),
        },
        header_area,
    );

    // Sidebar (hardware + compat)
    let selected_model = state
        .visible
        .get(state.selected_idx)
        .copied()
        .map(|idx| &state.models[idx]);

    let compat = selected_model.and_then(|m| crate::models::compat::evaluate(m, &state.hardware));

    frame.render_widget(
        Sidebar {
            hardware: &state.hardware,
            compat: compat.as_ref(),
            selected_name: selected_model.map(|m| m.name.as_str()),
        },
        sidebar_area,
    );

    // Model list (stateful — needs mutable list_state)
    frame.render_stateful_widget(
        ModelList {
            models: &state.models,
            visible: &state.visible,
            installed: &state.installed,
            search_query: &state.search_query,
            mode: state.mode,
            verdicts: &state.verdicts,
        },
        list_area,
        &mut state.list_state,
    );

    // Footer (keybindings, or temporary clipboard notification)
    let clip_msg = state
        .clipboard_msg
        .as_ref()
        .map(|(msg, _)| msg.clone());
    let footer = build_footer(state.mode, clip_msg);
    frame.render_widget(footer, footer_area);
}

fn build_footer(mode: AppMode, clipboard_msg: Option<String>) -> Paragraph<'static> {
    // Clipboard notification overrides the normal keybinding hint.
    if let Some(msg) = clipboard_msg {
        return Paragraph::new(Line::from(vec![
            " ".into(),
            Span::styled(
                msg,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]));
    }

    let version_span = Span::styled(
        format!(" v{APP_VERSION} "),
        Style::default().fg(Color::DarkGray),
    );

    // Claude Code-style keybinding display: [key] action
    let keys: Line = if mode == AppMode::Search {
        let mut spans: Vec<Span> = vec![
            " ".into(),
            "[esc]".bold().cyan(),
            " ".dark_gray(),
            t!("footer_esc").dark_gray(),
            "  ".into(),
            "[↑↓]".bold().cyan(),
            " ".dark_gray(),
            t!("footer_nav").dark_gray(),
        ];
        // version right-padded via a filler — we append it at the end and rely on the
        // terminal width to push it. A proper right-align needs a split layout, but for
        // footer simplicity we just append with a fixed spacer.
        spans.push(Span::raw("  "));
        spans.push(version_span);
        Line::from(spans)
    } else {
        Line::from(vec![
            " ".into(),
            "[q]".bold().cyan(),
            " ".dark_gray(),
            t!("footer_quit").dark_gray(),
            "  ".into(),
            "[/]".bold().cyan(),
            " ".dark_gray(),
            t!("footer_search").dark_gray(),
            "  ".into(),
            "[↑↓]".bold().cyan(),
            " ".dark_gray(),
            t!("footer_nav").dark_gray(),
            "  ".into(),
            "[L]".bold().cyan(),
            " ".dark_gray(),
            t!("footer_lang").dark_gray(),
            "  ".into(),
            "[c]".bold().cyan(),
            " ".dark_gray(),
            t!("footer_copy").dark_gray(),
            "  ".into(),
            version_span,
        ])
    };
    Paragraph::new(keys)
}
