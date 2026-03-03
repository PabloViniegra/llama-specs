use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Widget, Wrap},
};
use rust_i18n::t;

use crate::{
    hardware::HardwareInfo,
    models::{CompatResult, Verdict},
};

pub struct Sidebar<'a> {
    pub hardware: &'a HardwareInfo,
    pub compat: Option<&'a CompatResult>,
    pub selected_name: Option<&'a str>,
}

impl Widget for Sidebar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered().title(Span::styled(
            format!(" Hardware [{}] ", self.hardware.arch),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
        let inner = block.inner(area);
        block.render(area, buf);

        // Layout: RAM gauge, VRAM gauge, separator, compat details
        let [ram_area, vram_area, sep_area, detail_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(inner);

        // RAM bar
        render_gauge(
            ram_area,
            buf,
            &t!("sidebar_ram"),
            self.hardware.ram_available_mb,
            self.hardware.ram_total_mb,
            Color::Cyan,
        );

        // VRAM bar
        if let Some(gpu) = &self.hardware.gpu {
            let label = format!("{} ({})", t!("sidebar_vram"), gpu.name);
            render_gauge(
                vram_area,
                buf,
                &label,
                gpu.vram_free_mb,
                gpu.vram_total_mb,
                Color::Magenta,
            );
        } else {
            let no_gpu_label = t!("sidebar_no_gpu").to_string();
            Paragraph::new(no_gpu_label).dim().render(vram_area, buf);
        }

        // Horizontal separator
        let sep_line = "─".repeat(inner.width as usize);
        Paragraph::new(sep_line.dim()).render(sep_area, buf);

        // Compatibility detail panel
        if let (Some(name), Some(compat)) = (self.selected_name, self.compat) {
            render_compat(detail_area, buf, name, compat);
        } else {
            let hint = Paragraph::new("Select a model to see\ncompatibility details")
                .dim()
                .wrap(Wrap { trim: true });
            hint.render(detail_area, buf);
        }
    }
}

/// Renders a labelled progress gauge showing free / total usage.
fn render_gauge(
    area: Rect,
    buf: &mut Buffer,
    label: &str,
    free_mb: u64,
    total_mb: u64,
    color: Color,
) {
    let used_mb = total_mb.saturating_sub(free_mb);
    let ratio = if total_mb == 0 {
        0.0
    } else {
        (used_mb as f64 / total_mb as f64).clamp(0.0, 1.0)
    };

    let label_str = format!(
        "{label}  {:.1}/{:.1} GB",
        used_mb as f64 / 1024.0,
        total_mb as f64 / 1024.0
    );

    Gauge::default()
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(label_str)
        .render(area, buf);
}

/// Renders the compatibility verdict and memory breakdown.
fn render_compat(area: Rect, buf: &mut Buffer, name: &str, compat: &CompatResult) {
    let (verdict_str, verdict_color) = match compat.verdict {
        Verdict::Optimal => (t!("verdict_optimal"), Color::Green),
        Verdict::Hybrid => (t!("verdict_hybrid"), Color::Yellow),
        Verdict::Slow => (t!("verdict_slow"), Color::Rgb(255, 140, 0)),
        Verdict::Incompatible => (t!("verdict_incompatible"), Color::Red),
    };

    let desc = match compat.verdict {
        Verdict::Optimal => t!("verdict_optimal_desc"),
        Verdict::Hybrid => t!("verdict_hybrid_desc"),
        Verdict::Slow => t!("verdict_slow_desc"),
        Verdict::Incompatible => t!("verdict_incompatible_desc"),
    };

    let lines: Vec<Line> = vec![
        Line::from(vec![t!("sidebar_selected").dim(), ": ".dim()]),
        Line::from(Span::styled(
            format!("  {name}"),
            Style::default()
                .fg(Color::White)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(vec![
            t!("sidebar_estimated").dim(),
            format!(":  {:.1} GB", compat.estimated_mb as f64 / 1024.0).white(),
        ]),
        Line::raw(""),
        Line::from(vec![
            t!("sidebar_verdict").dim(),
            ": ".dim(),
            Span::styled(
                verdict_str.as_ref(),
                Style::default()
                    .fg(verdict_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {desc}"),
            Style::default()
                .fg(verdict_color)
                .add_modifier(ratatui::style::Modifier::DIM),
        )),
        Line::raw(""),
    ];

    // Memory breakdown lines
    let mut all_lines = lines;

    if compat.vram_used_mb > 0 {
        all_lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", t!("detail_vram_used")),
                Style::default().fg(Color::Magenta),
            ),
            format!("{:.1} GB", compat.vram_used_mb as f64 / 1024.0).white(),
        ]));
    }
    if compat.ram_used_mb > 0 {
        all_lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", t!("detail_ram_used")),
                Style::default().fg(Color::Cyan),
            ),
            format!("{:.1} GB", compat.ram_used_mb as f64 / 1024.0).white(),
        ]));
    }

    Paragraph::new(all_lines)
        .block(Block::new().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .render(area, buf);
}
