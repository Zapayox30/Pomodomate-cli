pub mod heatmap;
pub mod mascot;

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::{App, View};
use crate::timer::{TimerPhase, TimerStatus};

// ── Pomodomate color palette ─────────────────────────────────────────
/// Get phase-specific color
fn phase_color(phase: &TimerPhase, colors: &crate::theme::ThemeColors) -> Color {
    match phase {
        TimerPhase::Work => colors.tomato_red,
        TimerPhase::ShortBreak => colors.nature_green,
        TimerPhase::LongBreak => colors.accent_purple,
    }
}

/// How much UI fits in the current terminal size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Mascot + big digits + everything else
    Full,
    /// No mascot (short terminals, e.g. tmux splits)
    Compact,
    /// One-line timer + progress + keys (tiny terminals)
    Mini,
}

/// Pick a layout that fits without clipping the mascot mid-body.
fn layout_mode(_width: u16, height: u16, show_mascot: bool) -> LayoutMode {
    if height >= 38 && show_mascot {
        LayoutMode::Full
    } else if height >= 15 {
        LayoutMode::Compact
    } else {
        LayoutMode::Mini
    }
}

/// Whether the 3-row box-drawing digits fit horizontally.
fn fits_big_digits(width: u16) -> bool {
    width >= 34
}

/// Main draw function — renders the entire UI frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let mode = layout_mode(area.width, area.height, app.engine.config.show_mascot);

    // Background
    let bg_block = Block::default().style(Style::default().bg(app.theme_colors.dark_bg));
    frame.render_widget(bg_block, area);

    match (mode, app.current_view) {
        (LayoutMode::Mini, _) => draw_mini_view(frame, app, area),
        (_, View::Timer) => draw_timer_view(frame, app, area, mode),
        (_, View::Heatmap) => draw_heatmap_view(frame, app, area),
    }

    if app.show_help {
        draw_help_overlay(frame, area, &app.theme_colors);
    }
}

/// Render the main timer view, with the Domate mascot when it fits.
fn draw_timer_view(frame: &mut Frame, app: &App, area: Rect, mode: LayoutMode) {
    // Big digits need width and, in Compact, enough rows left over
    let use_big = fits_big_digits(area.width) && (mode == LayoutMode::Full || area.height >= 20);
    let timer_height = if use_big { 9 } else { 7 };

    if mode == LayoutMode::Full {
        let chunks = Layout::vertical([
            Constraint::Length(3),            // Header
            Constraint::Length(1),            // Spacer
            Constraint::Min(17),              // Mascot (full sprite)
            Constraint::Length(timer_height), // Timer
            Constraint::Length(2),            // Pomodoro counter
            Constraint::Length(3),            // Progress bar
            Constraint::Length(3),            // Footer
        ])
        .split(area);

        draw_header(frame, app, chunks[0]);
        mascot::draw_mascot(frame, app, chunks[2]);
        draw_timer_display(frame, app, chunks[3], use_big);
        draw_pomodoro_counter(frame, app, chunks[4]);
        draw_progress_bar(frame, app, chunks[5]);
        draw_footer(frame, app, chunks[6]);
    } else {
        let chunks = Layout::vertical([
            Constraint::Length(3),         // Header
            Constraint::Min(timer_height), // Timer
            Constraint::Length(2),         // Pomodoro counter
            Constraint::Length(3),         // Progress bar
            Constraint::Length(3),         // Footer
        ])
        .split(area);

        draw_header(frame, app, chunks[0]);
        draw_timer_display(frame, app, chunks[1], use_big);
        draw_pomodoro_counter(frame, app, chunks[2]);
        draw_progress_bar(frame, app, chunks[3]);
        draw_footer(frame, app, chunks[4]);
    }
}

/// One-line layout for tiny terminals: status, gauge, and keys.
fn draw_mini_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // Status line
        Constraint::Length(1), // Progress gauge
        Constraint::Length(1), // Keys
    ])
    .split(area);

    let pc = phase_color(&app.engine.timer.phase, &app.theme_colors);
    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", app.engine.timer.phase.label()),
            Style::default().fg(pc).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.engine.timer.remaining_display(),
            Style::default()
                .fg(app.theme_colors.soft_white)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  🍅 {}", app.engine.timer.pomodoros_completed),
            Style::default().fg(app.theme_colors.muted_text),
        ),
    ]));
    frame.render_widget(status, chunks[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(pc).bg(app.theme_colors.progress_bg))
        .ratio(app.engine.timer.progress())
        .label("");
    frame.render_widget(gauge, chunks[1]);

    let keys = Paragraph::new(Line::from(Span::styled(
        " space pause · s skip · q quit",
        Style::default().fg(app.theme_colors.muted_text),
    )));
    frame.render_widget(keys, chunks[2]);
}

/// Centered help overlay listing every keybinding. Any key closes it.
fn draw_help_overlay(frame: &mut Frame, area: Rect, colors: &crate::theme::ThemeColors) {
    let width = 44.min(area.width);
    let height = 14.min(area.height);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    frame.render_widget(ratatui::widgets::Clear, popup);

    let key_style = Style::default()
        .fg(colors.warm_yellow)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(colors.soft_white);
    let key_line = |key: &'static str, action: &'static str| {
        Line::from(vec![
            Span::styled(format!("  {:>7}  ", key), key_style),
            Span::styled(action, text_style),
        ])
    };

    let lines = vec![
        Line::from(""),
        key_line("space", "start / pause / resume"),
        key_line("r", "reset current phase"),
        key_line("s", "skip to next phase"),
        key_line("h", "toggle heatmap view"),
        key_line("+ / -", "add / remove one minute"),
        key_line("?", "toggle this help"),
        key_line("q", "quit (asks twice while running)"),
        Line::from(""),
        Line::from(Span::styled(
            "  press any key to close",
            Style::default()
                .fg(colors.muted_text)
                .add_modifier(Modifier::DIM),
        )),
    ];

    let help = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors.border_glow))
            .title(" ❔ Help ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(colors.dark_base)),
    );
    frame.render_widget(help, popup);
}

/// Render the heatmap view.
fn draw_heatmap_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header
        Constraint::Min(10),   // Heatmap
        Constraint::Length(3), // Footer
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    heatmap::draw_heatmap(frame, app, chunks[1]);
    draw_footer(frame, app, chunks[2]);
}

/// Header bar with app title and current state.
fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let pc = phase_color(&app.engine.timer.phase, &app.theme_colors);

    let status_icon = match app.engine.timer.status {
        TimerStatus::Running => ("▶", app.theme_colors.nature_green),
        TimerStatus::Paused => ("⏸", app.theme_colors.warm_yellow),
        TimerStatus::Idle => ("●", app.theme_colors.muted_text),
        TimerStatus::Completed => ("✓", app.theme_colors.nature_green),
    };

    let status_text = match app.engine.timer.status {
        TimerStatus::Running => "Running",
        TimerStatus::Paused => "Paused",
        TimerStatus::Idle => "Ready",
        TimerStatus::Completed => "Done!",
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("  🍅 ", Style::default().fg(app.theme_colors.tomato_red)),
        Span::styled(
            "Pomodomate",
            Style::default()
                .fg(app.theme_colors.tomato_red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ╱  ", Style::default().fg(app.theme_colors.border_dim)),
        Span::styled(
            app.engine.timer.phase.label(),
            Style::default().fg(pc).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ╱  ", Style::default().fg(app.theme_colors.border_dim)),
        Span::styled(status_icon.0, Style::default().fg(status_icon.1)),
        Span::styled(
            format!(" {}", status_text),
            Style::default().fg(app.theme_colors.soft_white),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(app.theme_colors.border_dim))
            .style(Style::default().bg(app.theme_colors.dark_base)),
    );

    frame.render_widget(header, area);
}

/// Timer display: box-drawing digits when there's room, plain MM:SS otherwise.
fn draw_timer_display(frame: &mut Frame, app: &App, area: Rect, use_big: bool) {
    let time_str = app.engine.timer.remaining_display();
    let pc = if app.engine.timer.is_last_minute() {
        app.theme_colors.warm_yellow
    } else {
        phase_color(&app.engine.timer.phase, &app.theme_colors)
    };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Phase label above timer
    lines.push(Line::from(vec![Span::styled(
        app.engine.timer.phase.label(),
        Style::default().fg(pc).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    if use_big {
        // Parse MM:SS
        let parts: Vec<&str> = time_str.split(':').collect();
        let (mins, secs) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("00", "00")
        };
        for big_line in render_big_time(mins, secs, pc) {
            lines.push(big_line);
        }
    } else {
        lines.push(Line::from(Span::styled(
            time_str,
            Style::default().fg(pc).add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));

    // Cycle info below
    let cycle_text = format!(
        "Pomodoro #{} ─ Cycle {}/{}",
        app.engine.timer.pomodoros_completed + 1,
        app.engine.timer.cycle_position + 1,
        app.engine.config.long_break_interval
    );
    lines.push(Line::from(Span::styled(
        cycle_text,
        Style::default().fg(app.theme_colors.muted_text),
    )));

    // Status hint
    let hint = match app.engine.timer.status {
        TimerStatus::Idle => "press [space] to start",
        TimerStatus::Paused => "press [space] to resume",
        TimerStatus::Completed => "press [space] for next",
        TimerStatus::Running => "",
    };
    if !hint.is_empty() {
        lines.push(Line::from(Span::styled(
            hint,
            Style::default()
                .fg(app.theme_colors.warm_yellow)
                .add_modifier(Modifier::DIM),
        )));
    }

    let timer_paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().bg(app.theme_colors.dark_bg)));

    frame.render_widget(timer_paragraph, area);
}

/// Render big ASCII-art numbers for the timer.
fn render_big_time(mins: &str, secs: &str, color: Color) -> Vec<Line<'static>> {
    // 3-line tall number font using block characters
    let digit_top = |d: char| -> &'static str {
        match d {
            '0' => "╭─╮",
            '1' => " ╷ ",
            '2' => "╶─╮",
            '3' => "╶─╮",
            '4' => "╷ ╷",
            '5' => "╭─╴",
            '6' => "╭─╴",
            '7' => "╶─╮",
            '8' => "╭─╮",
            '9' => "╭─╮",
            _ => "   ",
        }
    };

    let digit_mid = |d: char| -> &'static str {
        match d {
            '0' => "│ │",
            '1' => " │ ",
            '2' => "╭─╯",
            '3' => " ─┤",
            '4' => "╰─┤",
            '5' => "╰─╮",
            '6' => "├─╮",
            '7' => "  │",
            '8' => "├─┤",
            '9' => "╰─┤",
            _ => "   ",
        }
    };

    let digit_bot = |d: char| -> &'static str {
        match d {
            '0' => "╰─╯",
            '1' => " ╵ ",
            '2' => "╰─╴",
            '3' => "╶─╯",
            '4' => "  ╵",
            '5' => "╶─╯",
            '6' => "╰─╯",
            '7' => "  ╵",
            '8' => "╰─╯",
            '9' => "╶─╯",
            _ => "   ",
        }
    };

    let m_chars: Vec<char> = mins.chars().collect();
    let s_chars: Vec<char> = secs.chars().collect();

    let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let colon_style = Style::default().fg(color);
    let dim_style = Style::default().fg(color).add_modifier(Modifier::DIM);

    let m0 = m_chars.first().copied().unwrap_or('0');
    let m1 = m_chars.get(1).copied().unwrap_or('0');
    let s0 = s_chars.first().copied().unwrap_or('0');
    let s1 = s_chars.get(1).copied().unwrap_or('0');

    vec![
        Line::from(vec![
            Span::styled(format!(" {} {}", digit_top(m0), digit_top(m1)), style),
            Span::styled("   ", dim_style),
            Span::styled(format!("{} {} ", digit_top(s0), digit_top(s1)), style),
        ]),
        Line::from(vec![
            Span::styled(format!(" {} {}", digit_mid(m0), digit_mid(m1)), style),
            Span::styled(" ● ", colon_style),
            Span::styled(format!("{} {} ", digit_mid(s0), digit_mid(s1)), style),
        ]),
        Line::from(vec![
            Span::styled(format!(" {} {}", digit_bot(m0), digit_bot(m1)), style),
            Span::styled("   ", dim_style),
            Span::styled(format!("{} {} ", digit_bot(s0), digit_bot(s1)), style),
        ]),
    ]
}

/// Visual pomodoro counter — shows completed tomatoes.
fn draw_pomodoro_counter(frame: &mut Frame, app: &App, area: Rect) {
    let completed = app.engine.timer.pomodoros_completed;
    let cycle_pos = app.engine.timer.cycle_position;
    let interval = app.engine.config.long_break_interval;

    let mut spans: Vec<Span<'static>> = vec![Span::styled("  ", Style::default())];

    // Show tomatoes for current cycle
    for i in 0..interval {
        if i < cycle_pos
            || (i == cycle_pos
                && app.engine.timer.phase == TimerPhase::Work
                && app.engine.timer.status == TimerStatus::Completed)
        {
            // Completed in this cycle
            spans.push(Span::styled(" 🍅", Style::default()));
        } else if i == cycle_pos && app.engine.timer.phase == TimerPhase::Work {
            // Current (in progress)
            spans.push(Span::styled(
                " 🍅",
                Style::default().add_modifier(Modifier::DIM),
            ));
        } else {
            // Not yet
            spans.push(Span::styled(
                " ○ ",
                Style::default().fg(app.theme_colors.border_dim),
            ));
        }
    }

    // Total count
    spans.push(Span::styled(
        format!("    │ Total: {}", completed),
        Style::default().fg(app.theme_colors.muted_text),
    ));

    let counter = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(app.theme_colors.border_dim))
            .style(Style::default().bg(app.theme_colors.dark_base)),
    );

    frame.render_widget(counter, area);
}

/// Progress bar showing how much of the current phase is done.
fn draw_progress_bar(frame: &mut Frame, app: &App, area: Rect) {
    let pc = phase_color(&app.engine.timer.phase, &app.theme_colors);
    let progress = app.engine.timer.progress();

    let pct = (progress * 100.0) as u32;
    let label = format!("{}%  ─  {}", pct, app.engine.timer.remaining_display());

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme_colors.border_dim))
                .style(Style::default().bg(app.theme_colors.dark_base)),
        )
        .gauge_style(Style::default().fg(pc).bg(app.theme_colors.progress_bg))
        .ratio(progress)
        .label(Span::styled(
            label,
            Style::default()
                .fg(app.theme_colors.soft_white)
                .add_modifier(Modifier::BOLD),
        ));

    frame.render_widget(gauge, area);
}

/// Footer showing keybindings (or the quit confirmation warning).
fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let line = if app.quit_pending {
        Line::from(Span::styled(
            "  ⚠ timer is running — press q again to quit, any other key to stay",
            Style::default()
                .fg(app.theme_colors.warm_yellow)
                .add_modifier(Modifier::BOLD),
        ))
    } else if app.engine.paused_by_idle {
        // Explain the stopped clock instead of leaving them wondering.
        Line::from(Span::styled(
            "  ⏸ paused while you were away — press space to pick up where you left off",
            Style::default()
                .fg(app.theme_colors.warm_yellow)
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        let view_name = match app.current_view {
            View::Timer => "Heatmap",
            View::Heatmap => "Timer",
        };
        let key = |k: &'static str| {
            Span::styled(
                k,
                Style::default()
                    .fg(app.theme_colors.border_glow)
                    .add_modifier(Modifier::BOLD),
            )
        };
        let label = |t: String| Span::styled(t, Style::default().fg(app.theme_colors.muted_text));

        Line::from(vec![
            Span::styled("  ", Style::default()),
            key("space"),
            label(" pause  ".into()),
            key("r"),
            label(" reset  ".into()),
            key("s"),
            label(" skip  ".into()),
            key("h"),
            label(format!(" {}  ", view_name)),
            key("?"),
            label(" help  ".into()),
            Span::styled(
                "q",
                Style::default()
                    .fg(app.theme_colors.tomato_red)
                    .add_modifier(Modifier::BOLD),
            ),
            label(" quit".into()),
        ])
    };

    let footer = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(app.theme_colors.border_dim))
            .style(Style::default().bg(app.theme_colors.dark_base)),
    );

    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tall_terminals_get_the_full_layout_with_mascot() {
        assert_eq!(layout_mode(80, 45, true), LayoutMode::Full);
        assert_eq!(layout_mode(80, 38, true), LayoutMode::Full);
    }

    #[test]
    fn short_terminals_drop_the_mascot() {
        assert_eq!(layout_mode(80, 37, true), LayoutMode::Compact);
        assert_eq!(layout_mode(80, 15, true), LayoutMode::Compact);
    }

    #[test]
    fn tiny_terminals_get_the_one_line_mini_layout() {
        assert_eq!(layout_mode(80, 14, true), LayoutMode::Mini);
        assert_eq!(layout_mode(80, 5, true), LayoutMode::Mini);
    }

    #[test]
    fn layout_without_mascot_never_uses_full_mode() {
        assert_eq!(layout_mode(80, 45, false), LayoutMode::Compact);
        assert_eq!(layout_mode(80, 38, false), LayoutMode::Compact);
        assert_eq!(layout_mode(80, 10, false), LayoutMode::Mini);
    }

    #[test]
    fn narrow_terminals_never_use_big_digits() {
        assert!(fits_big_digits(34));
        assert!(!fits_big_digits(33));
    }
}
