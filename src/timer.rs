use std::time::Duration;

use crate::config::Config;

/// Represents which phase of the Pomodoro cycle we're in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerPhase {
    Work,
    ShortBreak,
    LongBreak,
}

impl TimerPhase {
    /// Human-readable label for UI display.
    pub fn label(&self) -> &'static str {
        match self {
            TimerPhase::Work => "🍅 Focus",
            TimerPhase::ShortBreak => "☕ Short Break",
            TimerPhase::LongBreak => "🌴 Long Break",
        }
    }

    /// Whether this phase is a break.
    #[allow(dead_code)]
    pub fn is_break(&self) -> bool {
        matches!(self, TimerPhase::ShortBreak | TimerPhase::LongBreak)
    }
}

/// Current status of the timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerStatus {
    /// Timer is actively counting down
    Running,
    /// Timer is paused by the user
    Paused,
    /// Timer hasn't been started yet / waiting for user
    Idle,
    /// Timer just completed a phase (transitional state)
    Completed,
}

/// The core Pomodoro timer state machine.
#[derive(Debug, Clone)]
pub struct Timer {
    /// Current phase (work, short break, long break)
    pub phase: TimerPhase,
    /// Current status
    pub status: TimerStatus,
    /// Time remaining in the current phase
    pub remaining: Duration,
    /// Total duration of the current phase (for progress calculation)
    pub total_duration: Duration,
    /// Number of completed work pomodoros in this session
    pub pomodoros_completed: u32,
    /// Position within the long-break cycle (0-based, resets after long break)
    pub cycle_position: u32,
    /// Animation tick counter (for mascot frames)
    pub tick: u64,
    /// Reference to timing config
    work_duration: Duration,
    short_break_duration: Duration,
    long_break_duration: Duration,
    long_break_interval: u32,
    auto_start_breaks: bool,
    auto_start_pomodoros: bool,
}

impl Timer {
    /// Create a new timer from the user's config.
    pub fn new(config: &Config) -> Self {
        let work_duration = Duration::from_secs(config.work_duration * 60);

        Self {
            phase: TimerPhase::Work,
            status: TimerStatus::Idle,
            remaining: work_duration,
            total_duration: work_duration,
            pomodoros_completed: 0,
            cycle_position: 0,
            tick: 0,
            work_duration,
            short_break_duration: Duration::from_secs(config.short_break * 60),
            long_break_duration: Duration::from_secs(config.long_break * 60),
            long_break_interval: config.long_break_interval,
            auto_start_breaks: config.auto_start_breaks,
            auto_start_pomodoros: config.auto_start_pomodoros,
        }
    }

    /// Advance the animation tick counter by one.
    pub fn animation_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    /// Advance the timer by one second. Returns `true` if the phase just completed.
    pub fn tick(&mut self) -> bool {

        if self.status != TimerStatus::Running {
            return false;
        }

        if self.remaining > Duration::from_secs(1) {
            self.remaining -= Duration::from_secs(1);
            false
        } else {
            self.remaining = Duration::ZERO;
            self.status = TimerStatus::Completed;
            true
        }
    }

    /// Toggle between paused and running.
    pub fn toggle_pause(&mut self) {
        match self.status {
            TimerStatus::Running => self.status = TimerStatus::Paused,
            TimerStatus::Paused => self.status = TimerStatus::Running,
            TimerStatus::Idle => self.status = TimerStatus::Running,
            TimerStatus::Completed => self.advance_to_next_phase(),
        }
    }

    /// Reset the current phase timer back to full.
    pub fn reset(&mut self) {
        self.remaining = self.total_duration;
        self.status = TimerStatus::Idle;
    }

    /// Skip ahead to the next phase.
    pub fn skip(&mut self) {
        self.advance_to_next_phase();
    }

    /// Transition to the next phase in the Pomodoro cycle.
    pub fn advance_to_next_phase(&mut self) {
        match self.phase {
            TimerPhase::Work => {
                self.pomodoros_completed += 1;
                self.cycle_position += 1;

                if self.cycle_position >= self.long_break_interval {
                    // Time for a long break
                    self.phase = TimerPhase::LongBreak;
                    self.remaining = self.long_break_duration;
                    self.total_duration = self.long_break_duration;
                    self.cycle_position = 0;
                } else {
                    // Short break
                    self.phase = TimerPhase::ShortBreak;
                    self.remaining = self.short_break_duration;
                    self.total_duration = self.short_break_duration;
                }

                self.status = if self.auto_start_breaks {
                    TimerStatus::Running
                } else {
                    TimerStatus::Idle
                };
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => {
                // Back to work
                self.phase = TimerPhase::Work;
                self.remaining = self.work_duration;
                self.total_duration = self.work_duration;

                self.status = if self.auto_start_pomodoros {
                    TimerStatus::Running
                } else {
                    TimerStatus::Idle
                };
            }
        }
    }

    /// Progress as a fraction from 0.0 (just started) to 1.0 (completed).
    pub fn progress(&self) -> f64 {
        if self.total_duration.is_zero() {
            return 1.0;
        }
        let elapsed = self.total_duration.as_secs_f64() - self.remaining.as_secs_f64();
        (elapsed / self.total_duration.as_secs_f64()).clamp(0.0, 1.0)
    }

    /// Format remaining time as "MM:SS".
    pub fn remaining_display(&self) -> String {
        let total_secs = self.remaining.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Whether we're in the last 60 seconds (for "last minute" mascot state).
    pub fn is_last_minute(&self) -> bool {
        self.status == TimerStatus::Running && self.remaining <= Duration::from_secs(60)
    }
}
