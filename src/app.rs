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
    /// Whether the help overlay is visible
    pub show_help: bool,
    /// True after a first `q` while the timer runs (waiting for confirmation)
    pub quit_pending: bool,
    /// Active theme colors
    pub theme_colors: crate::theme::ThemeColors,
    /// Timestamp when the current phase started (for session logging)
    phase_started_at: Option<chrono::DateTime<Utc>>,
}

impl App {
    /// Create a new App from the given config.
    pub fn new(config: Config) -> Result<Self> {
        let timer = Timer::new(&config);
        let storage = Storage::new()?;
        let theme_colors = crate::theme::ThemeColors::get(&config.theme, &config.custom_colors);

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
            show_help: false,
            quit_pending: false,
            theme_colors,
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
            KeyCode::Char(' ') => {
                // Record phase start time when starting for the first time
                let starting = self.timer.status == TimerStatus::Idle;
                if starting {
                    self.phase_started_at = Some(Utc::now());
                }
                self.timer.toggle_pause();
                if starting {
                    self.fire_phase_start();
                }
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
                // The phase ends here even though it was not completed, so
                // teardown hooks still get a chance to undo their setup.
                self.fire_phase_end(false);
                self.timer.skip();
                // Only start the clock on the new phase if it actually started
                if self.timer.status == TimerStatus::Running {
                    self.phase_started_at = Some(Utc::now());
                    self.fire_phase_start();
                }
            }
            KeyCode::Char('h') => {
                self.current_view = match self.current_view {
                    View::Timer => View::Heatmap,
                    View::Heatmap => View::Timer,
                };
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.timer.adjust(1);
            }
            KeyCode::Char('-') => {
                self.timer.adjust(-1);
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Don't lose a running session to a stray keypress
                if self.timer.status == TimerStatus::Running && !self.quit_pending {
                    self.quit_pending = true;
                } else {
                    self.should_quit = true;
                }
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

        // Fire the end hook while `timer.phase` still refers to the phase
        // that just finished.
        self.fire_phase_end(true);

        // Advance to next phase
        self.timer.advance_to_next_phase();

        // Record start of new phase if auto-starting
        if self.timer.status == TimerStatus::Running {
            self.phase_started_at = Some(Utc::now());
            self.fire_phase_start();
        }

        Ok(())
    }

    /// Stable identifier for a phase, used in stored sessions and hook env.
    fn phase_name(phase: TimerPhase) -> &'static str {
        match phase {
            TimerPhase::Work => "work",
            TimerPhase::ShortBreak => "short_break",
            TimerPhase::LongBreak => "long_break",
        }
    }

    /// Configured length of a phase in minutes.
    fn phase_duration(&self, phase: TimerPhase) -> u64 {
        match phase {
            TimerPhase::Work => self.config.work_duration,
            TimerPhase::ShortBreak => self.config.short_break,
            TimerPhase::LongBreak => self.config.long_break,
        }
    }

    /// Save a session record to local storage.
    fn save_session(&self, started: chrono::DateTime<Utc>, completed: bool) -> Result<()> {
        let session = Session::new(
            started,
            self.phase_duration(self.timer.phase),
            Self::phase_name(self.timer.phase),
            completed,
            self.config.tags.clone(),
        );
        self.storage.save_session(&session)?;
        Ok(())
    }

    /// Describe the current phase for a hook invocation.
    fn hook_context(&self, completed: bool) -> crate::hooks::HookContext {
        crate::hooks::HookContext {
            phase: Self::phase_name(self.timer.phase).to_string(),
            pomodoros: self.timer.pomodoros_completed,
            duration_minutes: self.phase_duration(self.timer.phase),
            tags: self.config.tags.join(","),
            completed,
        }
    }

    /// Fire the start hook for whichever phase just began.
    fn fire_phase_start(&self) {
        let hook = match self.timer.phase {
            TimerPhase::Work => &self.config.hooks.work_start,
            TimerPhase::ShortBreak | TimerPhase::LongBreak => &self.config.hooks.break_start,
        };
        if let Some(line) = hook {
            crate::hooks::spawn(line, &self.hook_context(false));
        }
    }

    /// Fire the end hook for the phase that is finishing. `completed` is false
    /// when the user skipped out of it.
    fn fire_phase_end(&self, completed: bool) {
        let hook = match self.timer.phase {
            TimerPhase::Work => &self.config.hooks.work_end,
            TimerPhase::ShortBreak | TimerPhase::LongBreak => &self.config.hooks.break_end,
        };
        if let Some(line) = hook {
            crate::hooks::spawn(line, &self.hook_context(completed));
        }
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
