use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::{DARK_BASE, NATURE_GREEN, SOFT_WHITE, TOMATO_RED};

/// Intensity levels for the heatmap cells.
fn intensity_color(count: u32) -> Color {
    match count {
        0 => Color::Rgb(44, 62, 80),       // Empty — dark blue-gray
        1 => Color::Rgb(72, 145, 83),       // Light green
        2 => Color::Rgb(57, 172, 57),       // Medium green
        3..=4 => Color::Rgb(39, 174, 96),   // Standard green
        _ => Color::Rgb(25, 210, 70),       // Intense green
    }
}

/// Draw a GitHub-style contribution heatmap of completed pomodoros.
pub fn draw_heatmap(frame: &mut Frame, app: &App, area: Rect) {
    // Load daily counts from storage (last 365 days)
    let daily_counts = match app.storage.daily_counts(365) {
        Ok(counts) => counts,
        Err(_) => {
            let error_msg = Paragraph::new("Failed to load session history")
                .alignment(Alignment::Center)
                .style(Style::default().fg(TOMATO_RED));
            frame.render_widget(error_msg, area);
            return;
        }
    };

    let mut lines: Vec<Line<'static>> = Vec::new();

    // Title
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "📊 Pomodoro Activity — Last 365 Days",
        Style::default()
            .fg(SOFT_WHITE)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Month labels row
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut month_spans: Vec<Span<'static>> = vec![Span::raw("     ")]; // Day label offset
    
    // Simplified month labels (evenly spaced)
    for month in &months {
        month_spans.push(Span::styled(
            format!("{:<6}", month),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(month_spans));

    // Day labels
    let day_labels = ["Mon", "   ", "Wed", "   ", "Fri", "   ", "Sun"];

    // Build the heatmap grid (7 rows × 52 columns)
    // Each row is a day of the week, each column is a week
    let total_days = daily_counts.len();
    let total_weeks = total_days.div_ceil(7);

    for (row_idx, day_label) in day_labels.iter().enumerate() {
        let mut row_spans: Vec<Span<'static>> = vec![
            Span::styled(format!("{} ", day_label), Style::default().fg(Color::DarkGray)),
        ];

        for week in 0..total_weeks.min(52) {
            let day_index = week * 7 + row_idx;
            if day_index < total_days {
                let (_date, count) = daily_counts[day_index];
                let color = intensity_color(count);
                row_spans.push(Span::styled("█ ", Style::default().fg(color)));
            } else {
                row_spans.push(Span::styled("  ", Style::default()));
            }
        }

        lines.push(Line::from(row_spans));
    }

    // Legend
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Less ", Style::default().fg(Color::DarkGray)),
        Span::styled("█ ", Style::default().fg(intensity_color(0))),
        Span::styled("█ ", Style::default().fg(intensity_color(1))),
        Span::styled("█ ", Style::default().fg(intensity_color(2))),
        Span::styled("█ ", Style::default().fg(intensity_color(3))),
        Span::styled("█ ", Style::default().fg(intensity_color(5))),
        Span::styled(" More", Style::default().fg(Color::DarkGray)),
    ]));

    // Summary stats
    let total_pomodoros: u32 = daily_counts.iter().map(|(_, c)| c).sum();
    let active_days = daily_counts.iter().filter(|(_, c)| *c > 0).count();
    let max_streak = calculate_streak(&daily_counts);

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!("  🍅 {} pomodoros", total_pomodoros),
            Style::default().fg(TOMATO_RED).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("📅 {} active days", active_days),
            Style::default().fg(NATURE_GREEN),
        ),
        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("🔥 {} day streak", max_streak),
            Style::default().fg(Color::Rgb(243, 156, 18)),
        ),
    ]));

    let heatmap = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" 🍅 Pomodomate Heatmap ")
                .title_alignment(Alignment::Center)
                .style(Style::default().bg(DARK_BASE)),
        );

    frame.render_widget(heatmap, area);
}

/// Calculate the longest consecutive-day streak of completed pomodoros.
fn calculate_streak(daily_counts: &[(chrono::NaiveDate, u32)]) -> u32 {
    let mut max_streak = 0u32;
    let mut current_streak = 0u32;

    for (_date, count) in daily_counts {
        if *count > 0 {
            current_streak += 1;
            max_streak = max_streak.max(current_streak);
        } else {
            current_streak = 0;
        }
    }

    max_streak
}
