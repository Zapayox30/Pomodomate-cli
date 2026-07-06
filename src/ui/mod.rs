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

/// Main draw function — renders the entire UI frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Background
    let bg_block = Block::default().style(Style::default().bg(DARK_BG));
    frame.render_widget(bg_block, area);

    match app.current_view {
        View::Timer => draw_timer_view(frame, app, area),
        View::Heatmap => draw_heatmap_view(frame, app, area),
    }
}

/// Render the main timer view with Domate mascot.
fn draw_timer_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header
        Constraint::Length(1), // Spacer
        Constraint::Min(8),    // Mascot (pixel art takes roughly 6-8 lines)
        Constraint::Length(5), // Timer (big numbers are 5 lines)
        Constraint::Length(1), // Spacer
        Constraint::Length(2), // Pomodoro counter
        Constraint::Length(3), // Progress bar
        Constraint::Length(3), // Footer
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    mascot::draw_mascot(frame, app, chunks[2]);
    draw_timer_display(frame, app, chunks[3]);
    draw_pomodoro_counter(frame, app, chunks[5]);
    draw_progress_bar(frame, app, chunks[6]);
    draw_footer(frame, app, chunks[7]);
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

/// Big ASCII-art timer display.
fn draw_timer_display(frame: &mut Frame, app: &App, area: Rect) {
    let time_str = app.timer.remaining_display();
    let pc = if app.timer.is_last_minute() {
        WARM_YELLOW
    } else {
        phase_color(&app.timer.phase)
    };

    // Parse MM:SS
    let parts: Vec<&str> = time_str.split(':').collect();
    let (mins, secs) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        ("00", "00")
    };

    // Build big number display (5 lines tall)
    let big_lines = render_big_time(mins, secs, pc);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(""));

    // Phase label above timer
    lines.push(Line::from(vec![Span::styled(
        app.timer.phase.label(),
        Style::default().fg(pc).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Big numbers
    for big_line in big_lines {
        lines.push(big_line);
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
        lines.push(Line::from(""));
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

/// Footer showing keybindings.
fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let view_name = match app.current_view {
        View::Timer => "Heatmap",
        View::Heatmap => "Timer",
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            "space",
            Style::default()
                .fg(BORDER_GLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" pause  ", Style::default().fg(MUTED_TEXT)),
        Span::styled(
            "r",
            Style::default()
                .fg(BORDER_GLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" reset  ", Style::default().fg(MUTED_TEXT)),
        Span::styled(
            "s",
            Style::default()
                .fg(BORDER_GLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" skip  ", Style::default().fg(MUTED_TEXT)),
        Span::styled(
            "h",
            Style::default()
                .fg(BORDER_GLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {}  ", view_name), Style::default().fg(MUTED_TEXT)),
        Span::styled(
            "q",
            Style::default().fg(TOMATO_RED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" quit", Style::default().fg(MUTED_TEXT)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(BORDER_DIM))
            .style(Style::default().bg(DARK_BASE)),
    );

    frame.render_widget(footer, area);
}
