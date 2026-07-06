use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::App;
use crate::timer::{TimerPhase, TimerStatus};
use crate::ui::DARK_BASE;

/// Visual states for the Domate mascot animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MascotState {
    /// Neutral waiting state (timer idle or paused)
    Idle,
    /// Welcome animation before the first work session of the day
    Sunrise,
    Working,
    ShortBreak,
    LongBreak,
    LastMinute,
    Completed,
}

impl MascotState {
    pub fn from_timer(app: &App) -> Self {
        if app.timer.status == TimerStatus::Completed {
            return MascotState::Completed;
        }
        if matches!(app.timer.status, TimerStatus::Idle | TimerStatus::Paused) {
            let awaiting_first_work = app.timer.phase == TimerPhase::Work
                && app.timer.pomodoros_completed == 0
                && app.first_session_today;
            return if awaiting_first_work {
                MascotState::Sunrise
            } else {
                MascotState::Idle
            };
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

pub fn draw_mascot(frame: &mut Frame, app: &App, area: Rect) {
    let state = MascotState::from_timer(app);
    let tick = app.timer.tick;

    let mascot_lines = generate_sprite(state, tick);

    let paragraph = Paragraph::new(mascot_lines)
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().bg(DARK_BASE)));

    frame.render_widget(paragraph, area);
}

// ── Color Palette based on Provided Image ────────────────────────────────────
const TOMATO_BODY: Color = Color::Rgb(238, 82, 63);      // Main Red
const TOMATO_SHADOW: Color = Color::Rgb(207, 59, 43);    // Shadow Red
const OUTLINE: Color = Color::Rgb(60, 20, 10);           // Deep Brown/Black Outline
const STEM: Color = Color::Rgb(61, 104, 34);             // Dark Green Stem
const LEAF: Color = Color::Rgb(111, 176, 62);            // Light Green Leaf
const GLASSES_RIM: Color = Color::Rgb(26, 26, 26);       // Sunglasses Rim
const GLASSES_LENS: Color = Color::Rgb(10, 10, 10);      // Dark Lens
const REFLECTION: Color = Color::Rgb(255, 255, 255);     // Lens Highlight
const MOUTH_DARK: Color = Color::Rgb(74, 26, 26);        // Inside Mouth
const TONGUE: Color = Color::Rgb(238, 123, 114);         // Pink Tongue
const ZZZ: Color = Color::Rgb(155, 89, 182);             // Sleep Zzz color
const SWEAT: Color = Color::Rgb(100, 200, 255);          // Last minute sweat color
const SWEAT_OUTLINE: Color = Color::Rgb(20, 80, 120);    // Sweat shadow

fn generate_sprite(state: MascotState, tick: u64) -> Vec<Line<'static>> {
    let frame_idx = (tick / 4) % 4; 
    let is_fast = state == MascotState::LastMinute;
    let bob_frame = if is_fast { (tick / 2) % 4 } else { frame_idx };

    // Vertical bobbing
    let is_bob_up = match state {
        MascotState::Working => bob_frame == 1 || bob_frame == 2,
        MascotState::ShortBreak | MascotState::LongBreak => (tick / 8).is_multiple_of(2),
        MascotState::LastMinute => bob_frame % 2 == 0,
        MascotState::Completed => bob_frame == 0 || bob_frame == 2,
        // Calm states barely move
        MascotState::Idle | MascotState::Sunrise => (tick / 16).is_multiple_of(2),
    };

    // The pixel grid perfectly matching the uploaded image (Domate with Sunglasses)
    let base_grid = vec![
        "         LL SSS         ", // 0
        "      LLLL  LS  LLLL    ", // 1
        "     LL      S     LL   ", // 2
        "   OOOOOOOOOOOOOOOOOO   ", // 3
        "  OOrrrrrrrrrrrrrrrrOO  ", // 4
        " OrrrrrEErrrrrrrrEErrrO ", // 5  E = Eyebrow
        " OrrrGGGGGGGGGGGGGGGGGrO", // 6  G = Glasses rim
        "OrrGGgWggggGGGGgWggggGGrO",// 7  g = lens, W = reflection
        "OrGGggggggGGGGggggggGGdO", // 8  d = shadow
        "OrGGggggggGGGGggggggGGdO", // 9
        "OrrrGGGGGGGrrrrGGGGGGGdO", // 10
        " OrrrrddrrMMMMMMrrddddrO", // 11 M = Mouth dark
        "  OOdddrrrMMTTMMrrddddOO", // 12 T = Tongue
        "    OOddddddddddddddOO  ", // 13
        "      OOOOOOOOOOOOOO    ", // 14
    ];

    let mut lines = Vec::new();

    if !is_bob_up {
        lines.push(Line::from(""));
    }

    for (row_idx, row) in base_grid.iter().enumerate() {
        let mut spans = Vec::new();
        let row_chars: Vec<char> = row.chars().collect();

        for (col_idx, &raw) in row_chars.iter().enumerate() {
            let c = modify_pixel(raw, row_idx, col_idx, state, frame_idx, tick);

            if let Some(color) = char_to_color(c) {
                // Colored "doble espacio" for a perfect square pixel
                spans.push(Span::styled("  ", Style::default().bg(color)));
            } else {
                // Effect particles and background
                match c {
                    'z' => spans.push(Span::styled(" z", Style::default().fg(ZZZ))),
                    'Z' => spans.push(Span::styled(" Z", Style::default().fg(ZZZ).add_modifier(ratatui::style::Modifier::BOLD))),
                    '!' => spans.push(Span::styled(" ⚡", Style::default())),
                    '*' => spans.push(Span::styled(" ✨", Style::default())),
                    's' => spans.push(Span::styled("  ", Style::default().bg(SWEAT_OUTLINE))),
                    'w' => spans.push(Span::styled("  ", Style::default().bg(SWEAT))),
                    _ => spans.push(Span::styled("  ", Style::default().bg(DARK_BASE))), // Transparent
                }
            }
        }
        lines.push(Line::from(spans));
    }

    if is_bob_up {
        lines.push(Line::from(""));
        lines.push(Line::from(""));
    } else {
        lines.push(Line::from(""));
    }

    lines
}

fn modify_pixel(c: char, row: usize, col: usize, state: MascotState, _frame: u64, abs_tick: u64) -> char {
    match state {
        MascotState::Working => {
            // Eyebrows bob slightly out of sync with body
            if c == 'E' && abs_tick % 8 >= 4 { return 'r'; }
            if c == 'r' && row == 4 && (col == 7 || col == 8 || col == 17 || col == 18) && abs_tick % 8 >= 4 { return 'E'; }
        }
        MascotState::ShortBreak => {
            // Sleeping: Eyebrows disappear, small O mouth
            if c == 'E' { return 'r'; }
            if c == 'T' { return 'M'; }
            if c == 'M' && (col <= 10 || col >= 15) { return 'd'; } // narrow mouth

            // Zzz particles
            let slow_frame = (abs_tick / 8) % 4;
            if slow_frame == 0 && row == 1 && col == 22 { return 'z'; }
            if slow_frame == 1 && row == 0 && col == 24 { return 'Z'; }
        }
        MascotState::LongBreak => {
            // Content/Chill: Sunglasses stay identical, big mouth, eyebrows standard
            if c == 'M' && col == 11 { return 'T'; }
        }
        MascotState::LastMinute => {
            // Panic: Eyebrows arched inverse? Hard to do. Flat mouth. Sweat drops.
            if c == 'T' { return 'r'; }
            if c == 'M' && row == 12 { return 'r'; } // Flat gritted teeth?
            
            // Sweat drops falling down the sides
            let fall = (abs_tick / 2) % 6;
            let sweat_row = 4 + fall as usize;
            
            // Left sweat drop
            if row == sweat_row && col == 2 { return 'w'; } // inside drop
            if row == sweat_row && (col == 1 || col == 3) { return 's'; } // outline
            
            // Right sweat drop (offset)
            let right_fall = ((abs_tick / 2) + 3) % 6;
            let right_sweat_row = 5 + right_fall as usize;
            if row == right_sweat_row && col == 22 { return 'w'; }
            if row == right_sweat_row && (col == 21 || col == 23) { return 's'; }
        }
        MascotState::Completed => {
            // Happy celebration!
            if (abs_tick / 2).is_multiple_of(2) {
                // Tongue wags
                if c == 'T' && col == 12 { return 'M'; }
                if c == 'M' && col == 14 { return 'T'; }
            }
            // Sparkles
            if (abs_tick / 2).is_multiple_of(2) && row == 2 && col == 1 { return '*'; }
            if !(abs_tick / 2).is_multiple_of(2) && row == 3 && col == 22 { return '*'; }
        }
        MascotState::Idle => {
            // Neutral waiting face: no eyebrows, small relaxed mouth
            if c == 'E' { return 'r'; }
            if c == 'T' { return 'M'; }
            if c == 'M' && (col <= 10 || col >= 15) { return 'd'; }
        }
        MascotState::Sunrise => {
            // Morning welcome: happy face plus slow sparkles greeting the day
            let slow = (abs_tick / 8) % 4;
            if slow == 0 && row == 1 && col == 1 { return '*'; }
            if slow == 1 && row == 0 && col == 22 { return '*'; }
            if slow == 2 && row == 2 && col == 23 { return '*'; }
            if slow == 3 && row == 3 && col == 0 { return '*'; }
        }
    }
    c
}

fn char_to_color(c: char) -> Option<Color> {
    match c {
        'O' => Some(OUTLINE),
        'r' => Some(TOMATO_BODY),
        'd' => Some(TOMATO_SHADOW),
        'E' => Some(OUTLINE),          // Eyebrows use the outline color
        'G' => Some(GLASSES_RIM),
        'g' => Some(GLASSES_LENS),
        'W' => Some(REFLECTION),
        'S' => Some(STEM),
        'L' => Some(LEAF),
        'M' => Some(MOUTH_DARK),
        'T' => Some(TONGUE),
        _ => None,
    }
}
