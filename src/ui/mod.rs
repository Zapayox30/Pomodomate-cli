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
pub const TOMATO_RED: Color = Color::Rgb(192, 57, 43);     // #C0392B
pub const NATURE_GREEN: Color = Color::Rgb(39, 174, 96);   // #27AE60
pub const CHEEK_PINK: Color = Color::Rgb(250, 219, 216);   // #FADBD8
pub const DARK_BASE: Color = Color::Rgb(28, 40, 51);       // #1C2833
pub const SOFT_WHITE: Color = Color::Rgb(236, 240, 241);   // #ECF0F1
pub const WARM_YELLOW: Color = Color::Rgb(243, 156, 18);   // #F39C12
pub const ACCENT_PURPLE: Color = Color::Rgb(142, 68, 173); // #8E44AD

/// Main draw function — renders the entire UI frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Background
    let bg_block = Block::default().style(Style::default().bg(DARK_BASE));
    frame.render_widget(bg_block, area);

    match app.current_view {
        View::Timer => draw_timer_view(frame, app, area),
        View::Heatmap => draw_heatmap_view(frame, app, area),
    }
}

/// Render the main timer view with Domate mascot.
fn draw_timer_view(frame: &mut Frame, app: &App, area: Rect) {
    // Layout: Header | Mascot + Timer | Footer
    let chunks = Layout::vertical([
        Constraint::Length(3),  // Header
        Constraint::Min(10),   // Main content (mascot + timer)
        Constraint::Length(3), // Progress bar
        Constraint::Length(3), // Footer (keybindings)
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);
    draw_main_content(frame, app, chunks[1]);
    draw_progress_bar(frame, app, chunks[2]);
    draw_footer(frame, app, chunks[3]);
}

/// Render the heatmap view.
fn draw_heatmap_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // Header
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
    let phase_color = match app.timer.phase {
        TimerPhase::Work => TOMATO_RED,
        TimerPhase::ShortBreak => NATURE_GREEN,
        TimerPhase::LongBreak => ACCENT_PURPLE,
    };

    let status_text = match app.timer.status {
        TimerStatus::Running => "▶ Running",
        TimerStatus::Paused => "⏸ Paused",
        TimerStatus::Idle => "⏹ Ready",
        TimerStatus::Completed => "✓ Done!",
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("  🍅 Pomodomate ", Style::default().fg(TOMATO_RED).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(app.timer.phase.label(), Style::default().fg(phase_color).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
        Span::styled(status_text, Style::default().fg(SOFT_WHITE)),
        Span::styled(
            format!("  │  🍅 × {} ", app.timer.pomodoros_completed),
            Style::default().fg(WARM_YELLOW),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(DARK_BASE)),
    );

    frame.render_widget(header, area);
}

/// Main content area: Mascot on the left, timer on the right.
fn draw_main_content(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::horizontal([
        Constraint::Percentage(50), // Mascot
        Constraint::Percentage(50), // Timer display
    ])
    .split(area);

    // Draw mascot
    mascot::draw_mascot(frame, app, chunks[0]);

    // Draw timer
    draw_timer_display(frame, app, chunks[1]);
}

/// Large timer display.
fn draw_timer_display(frame: &mut Frame, app: &App, area: Rect) {
    let time_str = app.timer.remaining_display();

    let phase_color = match app.timer.phase {
        TimerPhase::Work => TOMATO_RED,
        TimerPhase::ShortBreak => NATURE_GREEN,
        TimerPhase::LongBreak => ACCENT_PURPLE,
    };

    // Build large ASCII-art style numbers
    let timer_lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            &time_str,
            Style::default()
                .fg(if app.timer.is_last_minute() { WARM_YELLOW } else { phase_color })
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.timer.phase.label(),
            Style::default().fg(SOFT_WHITE),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Pomodoro #{}", app.timer.pomodoros_completed + 1),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            format!(
                "Cycle {}/{}",
                app.timer.cycle_position + 1,
                app.timer.pomodoros_completed
                    .checked_div(1)
                    .unwrap_or(0)
                    .max(app.timer.cycle_position + 1)
            ),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let timer_paragraph = Paragraph::new(timer_lines)
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().bg(DARK_BASE)));

    frame.render_widget(timer_paragraph, area);
}

/// Progress bar showing how much of the current phase is done.
fn draw_progress_bar(frame: &mut Frame, app: &App, area: Rect) {
    let phase_color = match app.timer.phase {
        TimerPhase::Work => TOMATO_RED,
        TimerPhase::ShortBreak => NATURE_GREEN,
        TimerPhase::LongBreak => ACCENT_PURPLE,
    };

    let progress = app.timer.progress();
    let label = format!(
        "{} — {}",
        app.timer.remaining_display(),
        app.timer.phase.label()
    );

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .style(Style::default().bg(DARK_BASE)),
        )
        .gauge_style(Style::default().fg(phase_color).bg(Color::Rgb(44, 62, 80)))
        .ratio(progress)
        .label(Span::styled(label, Style::default().fg(SOFT_WHITE).add_modifier(Modifier::BOLD)));

    frame.render_widget(gauge, area);
}

/// Footer showing keybindings.
fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let view_hint = match app.current_view {
        View::Timer => "[h] Heatmap",
        View::Heatmap => "[h] Timer",
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  [space]", Style::default().fg(WARM_YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(" Pause  ", Style::default().fg(SOFT_WHITE)),
        Span::styled("[r]", Style::default().fg(WARM_YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(" Reset  ", Style::default().fg(SOFT_WHITE)),
        Span::styled("[s]", Style::default().fg(WARM_YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(" Skip  ", Style::default().fg(SOFT_WHITE)),
        Span::styled("[h]", Style::default().fg(WARM_YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {}  ", view_hint.trim_start_matches("[h] ")), Style::default().fg(SOFT_WHITE)),
        Span::styled("[q]", Style::default().fg(TOMATO_RED).add_modifier(Modifier::BOLD)),
        Span::styled(" Quit", Style::default().fg(SOFT_WHITE)),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(DARK_BASE)),
    );

    frame.render_widget(footer, area);
}
