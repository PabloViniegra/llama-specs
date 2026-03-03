pub mod header;
pub mod model_list;
pub mod sidebar;

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::Paragraph,
};
use rust_i18n::t;

use crate::app::{AppMode, AppState};

use self::{header::Header, model_list::ModelList, sidebar::Sidebar};

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
                ratatui::style::Style::default()
                    .fg(ratatui::style::Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]));
    }

    let keys: Line = if mode == AppMode::Search {
        Line::from(vec![
            " ".into(),
            t!("footer_esc").bold().cyan(),
            "  ".into(),
            t!("footer_nav").dim(),
        ])
    } else {
        Line::from(vec![
            " ".into(),
            t!("footer_quit").bold().cyan(),
            "  ".into(),
            t!("footer_search").bold().cyan(),
            "  ".into(),
            t!("footer_nav").dim(),
            "  ".into(),
            t!("footer_lang").dim(),
            "  ".into(),
            t!("footer_copy").dim(),
        ])
    };
    Paragraph::new(keys)
}
