use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::App;
use crate::timer::{TimerPhase, TimerStatus};
use crate::ui::{CHEEK_PINK, DARK_BASE, NATURE_GREEN, SOFT_WHITE, TOMATO_RED, WARM_YELLOW};

/// Visual states for the Domate mascot animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MascotState {
    /// Normal working state — focused expression, slow blink
    Working,
    /// Short break — happy closed eyes, zzz floating
    ShortBreak,
    /// Long break — stretching, relaxed face
    LongBreak,
    /// Last 60 seconds — sweating, wide eyes, urgency
    LastMinute,
    /// Phase just completed — jumping, stars, euphoria
    Completed,
    /// First session of the day (future feature)
    #[allow(dead_code)]
    Dawn,
}

impl MascotState {
    /// Determine the mascot state from the current timer state.
    pub fn from_timer(app: &App) -> Self {
        if app.timer.status == TimerStatus::Completed {
            return MascotState::Completed;
        }
        if app.timer.is_last_minute() {
            return MascotState::LastMinute;
        }
        match app.timer.phase {
            TimerPhase::Work => MascotState::Working,
            TimerPhase::ShortBreak => MascotState::ShortBreak,
            TimerPhase::LongBreak => MascotState::LongBreak,
        }
    }
}

/// A single frame of mascot animation, as colored text lines.
/// (Will be used for more complex rendering in future versions)
#[allow(dead_code)]
struct MascotFrame {
    lines: Vec<Vec<(char, Color)>>,
}

/// Draw the Domate mascot in the given area.
pub fn draw_mascot(frame: &mut Frame, app: &App, area: Rect) {
    let state = MascotState::from_timer(app);
    let tick = app.timer.tick;

    // Select frame based on state and tick (2 frames per state for animation)
    let frame_index = (tick / 2) % 2; // Change frame every 2 ticks
    let mascot_lines = get_mascot_frame(state, frame_index as usize);

    let paragraph = Paragraph::new(mascot_lines)
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().bg(DARK_BASE)));

    frame.render_widget(paragraph, area);
}

/// Get the text lines for a specific mascot state and frame.
fn get_mascot_frame(state: MascotState, frame_idx: usize) -> Vec<Line<'static>> {
    match state {
        MascotState::Working => working_frames(frame_idx),
        MascotState::ShortBreak => short_break_frames(frame_idx),
        MascotState::LongBreak => long_break_frames(frame_idx),
        MascotState::LastMinute => last_minute_frames(frame_idx),
        MascotState::Completed => completed_frames(frame_idx),
        MascotState::Dawn => working_frames(frame_idx), // Placeholder
    }
}

// ── Frame generators ─────────────────────────────────────────────────
// Each function returns 2 frames of Domate in that state.
// Using Unicode block/circle characters + ANSI 24-bit colors.

fn working_frames(frame_idx: usize) -> Vec<Line<'static>> {
    let leaf = NATURE_GREEN;
    let body = TOMATO_RED;
    let cheek = CHEEK_PINK;
    let eye = SOFT_WHITE;

    if frame_idx == 0 {
        // Frame 1: Eyes open, focused
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("      🌿      ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("    ╭──────╮  ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled(" ◉", Style::default().fg(eye)),
                Span::styled("  ", Style::default().fg(body)),
                Span::styled("◉ ", Style::default().fg(eye)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled(" ‿‿ ", Style::default().fg(eye)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    ╰──────╯  ", Style::default().fg(body)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("   ◄ FOCUS ►  ", Style::default().fg(WARM_YELLOW)),
            ]),
            Line::from(""),
        ]
    } else {
        // Frame 2: Eyes blinking (half-closed)
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("      🌿      ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("    ╭──────╮  ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled(" ─", Style::default().fg(eye)),
                Span::styled("  ", Style::default().fg(body)),
                Span::styled("─ ", Style::default().fg(eye)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled(" ‿‿ ", Style::default().fg(eye)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    ╰──────╯  ", Style::default().fg(body)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("   ◄ FOCUS ►  ", Style::default().fg(WARM_YELLOW)),
            ]),
            Line::from(""),
        ]
    }
}

fn short_break_frames(frame_idx: usize) -> Vec<Line<'static>> {
    let leaf = NATURE_GREEN;
    let body = TOMATO_RED;
    let cheek = CHEEK_PINK;
    let eye = SOFT_WHITE;

    if frame_idx == 0 {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("      🌿    z ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("    ╭──────╮ z", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled(" ◡", Style::default().fg(eye)),
                Span::styled("  ", Style::default().fg(body)),
                Span::styled("◡ ", Style::default().fg(eye)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled(" ‿‿ ", Style::default().fg(eye)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    ╰──────╯  ", Style::default().fg(body)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    ☕ break   ", Style::default().fg(NATURE_GREEN)),
            ]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("      🌿  z   ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("    ╭──────╮z ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled(" ◡", Style::default().fg(eye)),
                Span::styled("  ", Style::default().fg(body)),
                Span::styled("◡ ", Style::default().fg(eye)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled(" ω  ", Style::default().fg(eye)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    ╰──────╯  ", Style::default().fg(body)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("    ☕ break   ", Style::default().fg(NATURE_GREEN)),
            ]),
            Line::from(""),
        ]
    }
}

fn long_break_frames(frame_idx: usize) -> Vec<Line<'static>> {
    let leaf = NATURE_GREEN;
    let body = TOMATO_RED;
    let cheek = CHEEK_PINK;
    let eye = SOFT_WHITE;
    let purple = Color::Rgb(142, 68, 173);

    if frame_idx == 0 {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("      🌿      ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("   ╭────────╮ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("  │", Style::default().fg(body)),
                Span::styled("  ◡", Style::default().fg(eye)),
                Span::styled("    ", Style::default().fg(body)),
                Span::styled("◡  ", Style::default().fg(eye)),
                Span::styled("│", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("  │", Style::default().fg(body)),
                Span::styled(" ●", Style::default().fg(cheek)),
                Span::styled(" ‿‿‿ ", Style::default().fg(eye)),
                Span::styled("● ", Style::default().fg(cheek)),
                Span::styled("│", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   ╰────────╯ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    \\(^_^)/   ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   🌴 relax 🌴 ", Style::default().fg(purple)),
            ]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("      🌿      ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("   ╭────────╮ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("  │", Style::default().fg(body)),
                Span::styled("  ◠", Style::default().fg(eye)),
                Span::styled("    ", Style::default().fg(body)),
                Span::styled("◠  ", Style::default().fg(eye)),
                Span::styled("│", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("  │", Style::default().fg(body)),
                Span::styled(" ●", Style::default().fg(cheek)),
                Span::styled(" ‿‿‿ ", Style::default().fg(eye)),
                Span::styled("● ", Style::default().fg(cheek)),
                Span::styled("│", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   ╰────────╯ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    /(^_^)\\   ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   🌴 relax 🌴 ", Style::default().fg(purple)),
            ]),
            Line::from(""),
        ]
    }
}

fn last_minute_frames(frame_idx: usize) -> Vec<Line<'static>> {
    let leaf = NATURE_GREEN;
    let body = TOMATO_RED;
    let cheek = CHEEK_PINK;
    let eye = SOFT_WHITE;

    if frame_idx == 0 {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("    💦🌿      ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("    ╭──────╮  ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled(" ⊙", Style::default().fg(eye)),
                Span::styled("  ", Style::default().fg(body)),
                Span::styled("⊙ ", Style::default().fg(eye)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled(" △△ ", Style::default().fg(eye)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    ╰──────╯  ", Style::default().fg(body)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ⚡ HURRY! ⚡ ", Style::default().fg(WARM_YELLOW)),
            ]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("      🌿💦    ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("    ╭──────╮  ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled(" ◎", Style::default().fg(eye)),
                Span::styled("  ", Style::default().fg(body)),
                Span::styled("◎ ", Style::default().fg(eye)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled(" ▽▽ ", Style::default().fg(eye)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    ╰──────╯  ", Style::default().fg(body)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ⚡ HURRY! ⚡ ", Style::default().fg(WARM_YELLOW)),
            ]),
            Line::from(""),
        ]
    }
}

fn completed_frames(frame_idx: usize) -> Vec<Line<'static>> {
    let leaf = NATURE_GREEN;
    let body = TOMATO_RED;
    let cheek = CHEEK_PINK;
    let eye = SOFT_WHITE;

    if frame_idx == 0 {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("   ✨ 🌿 ✨   ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("    ╭──────╮  ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled(" ★", Style::default().fg(WARM_YELLOW)),
                Span::styled("  ", Style::default().fg(body)),
                Span::styled("★ ", Style::default().fg(WARM_YELLOW)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled(" ▽▽ ", Style::default().fg(eye)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    ╰──────╯  ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("      \\○/     ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("  ✨ DONE! ✨  ", Style::default().fg(WARM_YELLOW)),
            ]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  ⭐  🌿  ⭐  ", Style::default().fg(leaf)),
            ]),
            Line::from(vec![
                Span::styled("    ╭──────╮  ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled(" ◉", Style::default().fg(WARM_YELLOW)),
                Span::styled("  ", Style::default().fg(body)),
                Span::styled("◉ ", Style::default().fg(WARM_YELLOW)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("   │", Style::default().fg(body)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled(" ▿▿ ", Style::default().fg(eye)),
                Span::styled("●", Style::default().fg(cheek)),
                Span::styled("│ ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("    ╰──────╯  ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("      /○\\     ", Style::default().fg(body)),
            ]),
            Line::from(vec![
                Span::styled("  🎉 DONE! 🎉 ", Style::default().fg(WARM_YELLOW)),
            ]),
            Line::from(""),
        ]
    }
}
