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
pub const TOMATO_RED: Color = Color::Rgb(192, 57, 43); // #C0392B
pub const NATURE_GREEN: Color = Color::Rgb(39, 174, 96); // #27AE60
#[allow(dead_code)]
pub const CHEEK_PINK: Color = Color::Rgb(250, 219, 216); // #FADBD8
pub const DARK_BASE: Color = Color::Rgb(28, 40, 51); // #1C2833
pub const SOFT_WHITE: Color = Color::Rgb(236, 240, 241); // #ECF0F1
pub const WARM_YELLOW: Color = Color::Rgb(243, 156, 18); // #F39C12
pub const ACCENT_PURPLE: Color = Color::Rgb(142, 68, 173); // #8E44AD

// Extended palette
const DARK_BG: Color = Color::Rgb(20, 30, 40); // Darker background
const BORDER_DIM: Color = Color::Rgb(52, 73, 94); // Subtle border
const BORDER_GLOW: Color = Color::Rgb(80, 110, 140); // Active border glow
const MUTED_TEXT: Color = Color::Rgb(127, 140, 141); // Muted gray text
const PROGRESS_BG: Color = Color::Rgb(44, 62, 80); // Progress bar background

/// Get phase-specific color
fn phase_color(phase: &TimerPhase) -> Color {
    match phase {
        TimerPhase::Work => TOMATO_RED,
        TimerPhase::ShortBreak => NATURE_GREEN,
        TimerPhase::LongBreak => ACCENT_PURPLE,
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
fn layout_mode(_width: u16, height: u16) -> LayoutMode {
    if height >= 38 {
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
    let mode = layout_mode(area.width, area.height);

    // Background
    let bg_block = Block::default().style(Style::default().bg(DARK_BG));
    frame.render_widget(bg_block, area);

    match (mode, app.current_view) {
        (LayoutMode::Mini, _) => draw_mini_view(frame, app, area),
        (_, View::Timer) => draw_timer_view(frame, app, area, mode),
        (_, View::Heatmap) => draw_heatmap_view(frame, app, area),
    }

    if app.show_help {
        draw_help_overlay(frame, area);
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

    let pc = phase_color(&app.timer.phase);
    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", app.timer.phase.label()),
            Style::default().fg(pc).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.timer.remaining_display(),
            Style::default().fg(SOFT_WHITE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  🍅 {}", app.timer.pomodoros_completed),
            Style::default().fg(MUTED_TEXT),
        ),
    ]));
    frame.render_widget(status, chunks[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(pc).bg(PROGRESS_BG))
        .ratio(app.timer.progress())
        .label("");
    frame.render_widget(gauge, chunks[1]);

    let keys = Paragraph::new(Line::from(Span::styled(
        " space pause · s skip · q quit",
        Style::default().fg(MUTED_TEXT),
    )));
    frame.render_widget(keys, chunks[2]);
}

/// Centered help overlay listing every keybinding. Any key closes it.
fn draw_help_overlay(frame: &mut Frame, area: Rect) {
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
        .fg(WARM_YELLOW)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(SOFT_WHITE);
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
            Style::default().fg(MUTED_TEXT).add_modifier(Modifier::DIM),
        )),
    ];

    let help = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_GLOW))
            .title(" ❔ Help ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(DARK_BASE)),
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
    let pc = phase_color(&app.timer.phase);

    let status_icon = match app.timer.status {
        TimerStatus::Running => ("▶", NATURE_GREEN),
        TimerStatus::Paused => ("⏸", WARM_YELLOW),
        TimerStatus::Idle => ("●", MUTED_TEXT),
        TimerStatus::Completed => ("✓", NATURE_GREEN),
    };

    let status_text = match app.timer.status {
        TimerStatus::Running => "Running",
        TimerStatus::Paused => "Paused",
        TimerStatus::Idle => "Ready",
        TimerStatus::Completed => "Done!",
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("  🍅 ", Style::default().fg(TOMATO_RED)),
        Span::styled(
            "Pomodomate",
            Style::default().fg(TOMATO_RED).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ╱  ", Style::default().fg(BORDER_DIM)),
        Span::styled(
            app.timer.phase.label(),
            Style::default().fg(pc).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ╱  ", Style::default().fg(BORDER_DIM)),
        Span::styled(status_icon.0, Style::default().fg(status_icon.1)),
        Span::styled(format!(" {}", status_text), Style::default().fg(SOFT_WHITE)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(DARK_BASE)),
    );

    frame.render_widget(header, area);
}

/// Timer display: box-drawing digits when there's room, plain MM:SS otherwise.
fn draw_timer_display(frame: &mut Frame, app: &App, area: Rect, use_big: bool) {
    let time_str = app.timer.remaining_display();
    let pc = if app.timer.is_last_minute() {
        WARM_YELLOW
    } else {
        phase_color(&app.timer.phase)
    };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Phase label above timer
    lines.push(Line::from(vec![Span::styled(
        app.timer.phase.label(),
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
        app.timer.pomodoros_completed + 1,
        app.timer.cycle_position + 1,
        app.config.long_break_interval
    );
    lines.push(Line::from(Span::styled(
        cycle_text,
        Style::default().fg(MUTED_TEXT),
    )));

    // Status hint
    let hint = match app.timer.status {
        TimerStatus::Idle => "press [space] to start",
        TimerStatus::Paused => "press [space] to resume",
        TimerStatus::Completed => "press [space] for next",
        TimerStatus::Running => "",
    };
    if !hint.is_empty() {
        lines.push(Line::from(Span::styled(
            hint,
            Style::default().fg(WARM_YELLOW).add_modifier(Modifier::DIM),
        )));
    }

    let timer_paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().bg(DARK_BG)));

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
    let completed = app.timer.pomodoros_completed;
    let cycle_pos = app.timer.cycle_position;
    let interval = app.config.long_break_interval;

    let mut spans: Vec<Span<'static>> = vec![Span::styled("  ", Style::default())];

    // Show tomatoes for current cycle
    for i in 0..interval {
        if i < cycle_pos
            || (i == cycle_pos
                && app.timer.phase == TimerPhase::Work
                && app.timer.status == TimerStatus::Completed)
        {
            // Completed in this cycle
            spans.push(Span::styled(" 🍅", Style::default()));
        } else if i == cycle_pos && app.timer.phase == TimerPhase::Work {
            // Current (in progress)
            spans.push(Span::styled(
                " 🍅",
                Style::default().add_modifier(Modifier::DIM),
            ));
        } else {
            // Not yet
            spans.push(Span::styled(" ○ ", Style::default().fg(BORDER_DIM)));
        }
    }

    // Total count
    spans.push(Span::styled(
        format!("    │ Total: {}", completed),
        Style::default().fg(MUTED_TEXT),
    ));

    let counter = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(DARK_BASE)),
    );

    frame.render_widget(counter, area);
}

/// Progress bar showing how much of the current phase is done.
fn draw_progress_bar(frame: &mut Frame, app: &App, area: Rect) {
    let pc = phase_color(&app.timer.phase);
    let progress = app.timer.progress();

    let pct = (progress * 100.0) as u32;
    let label = format!("{}%  ─  {}", pct, app.timer.remaining_display());

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER_DIM))
                .style(Style::default().bg(DARK_BASE)),
        )
        .gauge_style(Style::default().fg(pc).bg(PROGRESS_BG))
        .ratio(progress)
        .label(Span::styled(
            label,
            Style::default().fg(SOFT_WHITE).add_modifier(Modifier::BOLD),
        ));

    frame.render_widget(gauge, area);
}

/// Footer showing keybindings (or the quit confirmation warning).
fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let line = if app.quit_pending {
        Line::from(Span::styled(
            "  ⚠ timer is running — press q again to quit, any other key to stay",
            Style::default()
                .fg(WARM_YELLOW)
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
                    .fg(BORDER_GLOW)
                    .add_modifier(Modifier::BOLD),
            )
        };
        let label = |t: String| Span::styled(t, Style::default().fg(MUTED_TEXT));

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
                Style::default().fg(TOMATO_RED).add_modifier(Modifier::BOLD),
            ),
            label(" quit".into()),
        ])
    };

    let footer = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(DARK_BASE)),
    );

    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tall_terminals_get_the_full_layout_with_mascot() {
        assert_eq!(layout_mode(80, 45), LayoutMode::Full);
        assert_eq!(layout_mode(80, 38), LayoutMode::Full);
    }

    #[test]
    fn short_terminals_drop_the_mascot() {
        assert_eq!(layout_mode(80, 37), LayoutMode::Compact);
        assert_eq!(layout_mode(80, 15), LayoutMode::Compact);
    }

    #[test]
    fn tiny_terminals_get_the_one_line_mini_layout() {
        assert_eq!(layout_mode(80, 14), LayoutMode::Mini);
        assert_eq!(layout_mode(80, 5), LayoutMode::Mini);
    }

    #[test]
    fn narrow_terminals_never_use_big_digits() {
        assert!(fits_big_digits(34));
        assert!(!fits_big_digits(33));
    }
}
