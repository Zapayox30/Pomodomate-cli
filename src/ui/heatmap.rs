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
        0 => Color::Rgb(44, 62, 80),      // Empty — dark blue-gray
        1 => Color::Rgb(72, 145, 83),     // Light green
        2 => Color::Rgb(57, 172, 57),     // Medium green
        3..=4 => Color::Rgb(39, 174, 96), // Standard green
        _ => Color::Rgb(25, 210, 70),     // Intense green
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
        Style::default().fg(SOFT_WHITE).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Build the grid: columns are Mon-Sun weeks, aligned to real weekdays.
    // Show at most the latest 52 weeks so it fits a normal terminal.
    let full_grid = build_grid(&daily_counts);
    let skip = full_grid.len().saturating_sub(52);
    let grid = &full_grid[skip..];

    // Month labels: mark the column where a new month starts.
    lines.push(Line::from(vec![
        Span::raw("    "), // Day label offset
        Span::styled(month_label_row(grid), Style::default().fg(Color::DarkGray)),
    ]));

    // Day labels
    let day_labels = ["Mon", "   ", "Wed", "   ", "Fri", "   ", "Sun"];

    for (row_idx, day_label) in day_labels.iter().enumerate() {
        let mut row_spans: Vec<Span<'static>> = vec![Span::styled(
            format!("{} ", day_label),
            Style::default().fg(Color::DarkGray),
        )];

        for week in grid {
            match week[row_idx] {
                Some((_date, count)) => {
                    let color = intensity_color(count);
                    row_spans.push(Span::styled("█ ", Style::default().fg(color)));
                }
                None => row_spans.push(Span::styled("  ", Style::default())),
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
    let max_streak = crate::storage::calculate_streak(&daily_counts);

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

    let heatmap = Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" 🍅 Pomodomate Heatmap ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(DARK_BASE)),
    );

    frame.render_widget(heatmap, area);
}

type Cell = Option<(chrono::NaiveDate, u32)>;

/// Arrange daily counts (oldest first) into GitHub-style week columns,
/// where each column is a Monday-to-Sunday week and each date sits in the
/// row of its actual weekday.
fn build_grid(daily_counts: &[(chrono::NaiveDate, u32)]) -> Vec<[Cell; 7]> {
    use chrono::Datelike;

    let Some(&(first, _)) = daily_counts.first() else {
        return Vec::new();
    };

    // Monday of the week containing the first date anchors column 0.
    let anchor = first - chrono::Duration::days(first.weekday().num_days_from_monday() as i64);

    let mut grid: Vec<[Cell; 7]> = Vec::new();
    for &(date, count) in daily_counts {
        let col = ((date - anchor).num_days() / 7) as usize;
        let row = date.weekday().num_days_from_monday() as usize;
        if grid.len() <= col {
            grid.resize(col + 1, [None; 7]);
        }
        grid[col][row] = Some((date, count));
    }

    grid
}

/// Build the month label row: each week column is 2 chars wide, and a month
/// abbreviation is written at the first column belonging to that month.
fn month_label_row(grid: &[[Cell; 7]]) -> String {
    use chrono::Datelike;

    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let mut row = vec![' '; grid.len() * 2];
    let mut prev_month = 0u32;
    let mut next_free = 0usize;

    for (col, week) in grid.iter().enumerate() {
        let Some((date, _)) = week.iter().flatten().next() else {
            continue;
        };
        if date.month() != prev_month {
            prev_month = date.month();
            let pos = col * 2;
            // Leave a gap between labels so they never run together
            if pos >= next_free {
                for (i, ch) in MONTHS[date.month0() as usize].chars().enumerate() {
                    if pos + i < row.len() {
                        row[pos + i] = ch;
                    }
                }
                next_free = pos + 4;
            }
        }
    }

    row.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn grid_aligns_dates_to_real_weekdays() {
        // 2025-01-01 was a Wednesday; the range Wed Jan 1 .. Tue Jan 7
        // must span two Mon-Sun columns with the right slots empty.
        let days: Vec<_> = (1..=7).map(|i| (d(2025, 1, i), i)).collect();
        let grid = build_grid(&days);

        assert_eq!(grid.len(), 2);
        assert!(grid[0][0].is_none(), "Mon of week 1 predates the range");
        assert!(grid[0][1].is_none(), "Tue of week 1 predates the range");
        assert_eq!(
            grid[0][2],
            Some((d(2025, 1, 1), 1)),
            "Jan 1 lands on Wednesday"
        );
        assert_eq!(
            grid[0][6],
            Some((d(2025, 1, 5), 5)),
            "Jan 5 lands on Sunday"
        );
        assert_eq!(
            grid[1][0],
            Some((d(2025, 1, 6), 6)),
            "Jan 6 starts the next week"
        );
        assert_eq!(grid[1][1], Some((d(2025, 1, 7), 7)));
        assert!(grid[1][2].is_none(), "rest of week 2 is beyond the range");
    }

    #[test]
    fn streak_counts_longest_consecutive_run() {
        let counts: Vec<(NaiveDate, u32)> = [1, 0, 2, 1, 3, 0, 1]
            .iter()
            .enumerate()
            .map(|(i, c)| (d(2025, 1, i as u32 + 1), *c))
            .collect();
        assert_eq!(crate::storage::calculate_streak(&counts), 3);
    }

    #[test]
    fn streak_is_zero_without_activity() {
        let counts = vec![(d(2025, 1, 1), 0), (d(2025, 1, 2), 0)];
        assert_eq!(crate::storage::calculate_streak(&counts), 0);
    }
}
