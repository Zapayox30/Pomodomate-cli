use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};

use crate::config::Config;
use crate::engine::Engine;
use crate::timer::TimerStatus;
use crate::ui;

/// Which view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Timer,
    Heatmap,
}

/// The terminal front-end.
///
/// All pomodoro rules live in [`Engine`]; `App` only translates key presses
/// into engine calls and decides what to draw.
pub struct App {
    /// Timer, history, hooks and notifications.
    pub engine: Engine,
    /// Current view (timer or heatmap)
    pub current_view: View,
    /// Whether the app should exit
    pub should_quit: bool,
    /// Whether the help overlay is visible
    pub show_help: bool,
    /// True after a first `q` while the timer runs (waiting for confirmation)
    pub quit_pending: bool,
    /// Active theme colors
    pub theme_colors: crate::theme::ThemeColors,
    /// Typeface for the big clock, cycled with `d`
    pub digit_style: crate::ui::digits::DigitStyle,
}

impl App {
    /// Create a new App from the given config.
    pub fn new(config: Config) -> Result<Self> {
        let theme_colors = crate::theme::ThemeColors::get(&config.theme, &config.custom_colors);
        let digit_style = crate::ui::digits::DigitStyle::from_name(&config.digit_style);
        let engine = Engine::new(config)?;

        Ok(Self {
            engine,
            current_view: View::Timer,
            should_quit: false,
            show_help: false,
            quit_pending: false,
            theme_colors,
            digit_style,
        })
    }

    /// Main application loop — runs until the user quits.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let tick_rate = Duration::from_millis(125); // 8 FPS
        let mut last_tick = Instant::now();
        let mut last_timer_update = Instant::now();

        // Absent on X11 or compositors without ext-idle-notify-v1, in which
        // case the timer simply never pauses itself.
        let idle_events =
            crate::idle::watch(Duration::from_secs(self.engine.config.idle_timeout * 60));

        while !self.should_quit {
            if let Some(events) = &idle_events {
                while let Ok(event) = events.try_recv() {
                    self.engine.handle_idle(event);
                }
            }

            // Draw
            terminal.draw(|frame: &mut Frame| {
                ui::draw(frame, self);
            })?;

            // Wait for events or tick timeout
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code)?;
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.engine.timer.animation_tick();
                last_tick = Instant::now();
            }

            // Timer tick every second
            if last_timer_update.elapsed() >= Duration::from_secs(1) {
                self.engine.tick()?;
                last_timer_update += Duration::from_secs(1);
            }
        }

        Ok(())
    }

    /// Handle a key press event.
    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        // The help overlay swallows the next key press
        if self.show_help {
            self.show_help = false;
            return Ok(());
        }

        // Any key except q/Esc cancels a pending quit confirmation
        if !matches!(key, KeyCode::Char('q') | KeyCode::Esc) {
            self.quit_pending = false;
        }

        match key {
            KeyCode::Char(' ') => self.engine.toggle(),
            KeyCode::Char('r') => self.engine.reset(),
            KeyCode::Char('s') => self.engine.skip()?,
            KeyCode::Char('h') => {
                self.current_view = match self.current_view {
                    View::Timer => View::Heatmap,
                    View::Heatmap => View::Timer,
                };
            }
            KeyCode::Char('d') => {
                self.digit_style = self.digit_style.next();
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.engine.adjust(1);
            }
            KeyCode::Char('-') => {
                self.engine.adjust(-1);
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Don't lose a running session to a stray keypress
                if self.engine.timer.status == TimerStatus::Running && !self.quit_pending {
                    self.quit_pending = true;
                } else {
                    // Record whatever was worked before leaving.
                    self.engine.abandon_phase();
                    self.should_quit = true;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
