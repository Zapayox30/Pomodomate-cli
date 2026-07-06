use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};

use crate::config::Config;
use crate::storage::{Session, Storage};
use crate::timer::{Timer, TimerPhase, TimerStatus};
use crate::ui;

/// Which view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Timer,
    Heatmap,
}

/// Global application state.
pub struct App {
    /// The Pomodoro timer state machine
    pub timer: Timer,
    /// User configuration
    pub config: Config,
    /// Local session storage
    pub storage: Storage,
    /// Current view (timer or heatmap)
    pub current_view: View,
    /// Whether the app should exit
    pub should_quit: bool,
    /// True if no work session has been completed yet today (sunrise mascot)
    pub first_session_today: bool,
    /// Timestamp when the current phase started (for session logging)
    phase_started_at: Option<chrono::DateTime<Utc>>,
}

impl App {
    /// Create a new App from the given config.
    pub fn new(config: Config) -> Result<Self> {
        let timer = Timer::new(&config);
        let storage = Storage::new()?;

        let first_session_today = storage
            .daily_counts(1)
            .map(|counts| counts.iter().all(|(_, n)| *n == 0))
            .unwrap_or(true);

        Ok(Self {
            timer,
            config,
            storage,
            current_view: View::Timer,
            should_quit: false,
            first_session_today,
            phase_started_at: None,
        })
    }

    /// Main application loop — runs until the user quits.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let tick_rate = Duration::from_millis(125); // 8 FPS
        let mut last_tick = Instant::now();
        let mut last_timer_update = Instant::now();

        while !self.should_quit {
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
                self.timer.animation_tick();
                last_tick = Instant::now();
            }

            // Timer tick every second
            if last_timer_update.elapsed() >= Duration::from_secs(1) {
                let phase_completed = self.timer.tick();

                if phase_completed {
                    self.on_phase_complete()?;
                }

                last_timer_update += Duration::from_secs(1);
            }
        }

        Ok(())
    }

    /// Handle a key press event.
    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char(' ') => {
                // Record phase start time when starting for the first time
                if self.timer.status == TimerStatus::Idle {
                    self.phase_started_at = Some(Utc::now());
                }
                self.timer.toggle_pause();
            }
            KeyCode::Char('r') => {
                self.timer.reset();
                self.phase_started_at = None;
            }
            KeyCode::Char('s') => {
                // Save incomplete session before skipping
                if let Some(started) = self.phase_started_at.take() {
                    self.save_session(started, false)?;
                }
                self.timer.skip();
                // Only start the clock on the new phase if it actually started
                if self.timer.status == TimerStatus::Running {
                    self.phase_started_at = Some(Utc::now());
                }
            }
            KeyCode::Char('h') => {
                self.current_view = match self.current_view {
                    View::Timer => View::Heatmap,
                    View::Heatmap => View::Timer,
                };
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
        Ok(())
    }

    /// Called when a timer phase completes naturally.
    fn on_phase_complete(&mut self) -> Result<()> {
        // Save the completed session
        if let Some(started) = self.phase_started_at.take() {
            self.save_session(started, true)?;
        }

        // Mark work completed for today (disables the sunrise animation)
        if self.timer.phase == TimerPhase::Work {
            self.first_session_today = false;
        }

        // Send desktop notification
        if self.config.notifications {
            self.send_notification();
        }

        // Audible cue, independent of desktop notifications
        if self.config.sound {
            Self::ring_bell();
        }

        // Advance to next phase
        self.timer.advance_to_next_phase();

        // Record start of new phase if auto-starting
        if self.timer.status == TimerStatus::Running {
            self.phase_started_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Save a session record to local storage.
    fn save_session(&self, started: chrono::DateTime<Utc>, completed: bool) -> Result<()> {
        let phase_str = match self.timer.phase {
            TimerPhase::Work => "work",
            TimerPhase::ShortBreak => "short_break",
            TimerPhase::LongBreak => "long_break",
        };

        let duration = match self.timer.phase {
            TimerPhase::Work => self.config.work_duration,
            TimerPhase::ShortBreak => self.config.short_break,
            TimerPhase::LongBreak => self.config.long_break,
        };

        let session = Session::new(started, duration, phase_str, completed);
        self.storage.save_session(&session)?;
        Ok(())
    }

    /// Send a desktop notification for the current phase transition.
    fn send_notification(&self) {
        let (summary, body) = match self.timer.phase {
            TimerPhase::Work => (
                "🍅 Pomodoro Complete!",
                format!(
                    "Great focus session! #{} done. Time for a break.",
                    self.timer.pomodoros_completed
                ),
            ),
            TimerPhase::ShortBreak => (
                "☕ Break Over!",
                "Short break is done. Ready to focus again?".to_string(),
            ),
            TimerPhase::LongBreak => (
                "🌴 Long Break Over!",
                "Feeling refreshed? Let's get back to work!".to_string(),
            ),
        };

        let mut notification = notify_rust::Notification::new();
        notification
            .summary(summary)
            .body(&body)
            .icon("pomodomate")
            .timeout(notify_rust::Timeout::Milliseconds(5000));

        if self.config.sound {
            // XDG sound theme name; honored by notification daemons with sound support
            notification.sound_name("complete");
        }

        // Fire-and-forget notification — don't crash if it fails
        let _ = notification.show();
    }

    /// Ring the terminal bell (BEL). Works even when notifications are off.
    fn ring_bell() {
        use std::io::Write;
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(b"\x07");
        let _ = stdout.flush();
    }
}
