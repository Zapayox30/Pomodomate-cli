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

    /// Skip ahead to the next phase without crediting the current one.
    pub fn skip(&mut self) {
        self.transition(false);
    }

    /// Transition to the next phase after the current one completed naturally.
    pub fn advance_to_next_phase(&mut self) {
        self.transition(true);
    }

    /// Move to the next phase. Only a naturally completed work phase counts
    /// toward `pomodoros_completed` and the long-break cycle.
    fn transition(&mut self, work_completed: bool) {
        match self.phase {
            TimerPhase::Work => {
                if work_completed {
                    self.pomodoros_completed += 1;
                    self.cycle_position += 1;
                }

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

    /// Add or subtract whole minutes from the current phase, keeping at
    /// least one minute on the clock and progress within [0, 1].
    pub fn adjust(&mut self, minutes: i64) {
        let delta = Duration::from_secs(60);
        if minutes > 0 {
            self.remaining += delta;
            self.total_duration += delta;
        } else if minutes < 0 && self.remaining > delta {
            self.remaining = (self.remaining - delta).max(delta);
        }
        if self.total_duration < self.remaining {
            self.total_duration = self.remaining;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn timer() -> Timer {
        Timer::new(&Config::default())
    }

    /// Run the current phase down to zero as if the full duration elapsed.
    fn complete_phase(t: &mut Timer) {
        t.status = TimerStatus::Running;
        t.remaining = Duration::from_secs(1);
        assert!(t.tick(), "phase should complete");
    }

    #[test]
    fn starts_idle_with_full_work_duration() {
        let t = timer();
        assert_eq!(t.phase, TimerPhase::Work);
        assert_eq!(t.status, TimerStatus::Idle);
        assert_eq!(t.remaining, Duration::from_secs(25 * 60));
        assert_eq!(t.pomodoros_completed, 0);
    }

    #[test]
    fn tick_only_counts_down_while_running() {
        let mut t = timer();
        assert!(!t.tick());
        assert_eq!(
            t.remaining,
            Duration::from_secs(25 * 60),
            "idle timer must not advance"
        );

        t.toggle_pause(); // Idle -> Running
        assert!(!t.tick());
        assert_eq!(t.remaining, Duration::from_secs(25 * 60 - 1));

        t.toggle_pause(); // Running -> Paused
        assert!(!t.tick());
        assert_eq!(
            t.remaining,
            Duration::from_secs(25 * 60 - 1),
            "paused timer must not advance"
        );
    }

    #[test]
    fn completing_work_counts_pomodoro_and_starts_short_break() {
        let mut t = timer();
        complete_phase(&mut t);
        assert_eq!(t.status, TimerStatus::Completed);

        t.advance_to_next_phase();
        assert_eq!(t.pomodoros_completed, 1);
        assert_eq!(t.phase, TimerPhase::ShortBreak);
        assert_eq!(t.remaining, Duration::from_secs(5 * 60));
        // auto_start_breaks defaults to true
        assert_eq!(t.status, TimerStatus::Running);
    }

    #[test]
    fn long_break_after_completing_interval_works() {
        let mut t = timer();
        for i in 1..=4 {
            assert_eq!(t.phase, TimerPhase::Work);
            complete_phase(&mut t);
            t.advance_to_next_phase();
            assert_eq!(t.pomodoros_completed, i);
            if i < 4 {
                assert_eq!(t.phase, TimerPhase::ShortBreak);
                complete_phase(&mut t);
                t.advance_to_next_phase();
            }
        }
        assert_eq!(t.phase, TimerPhase::LongBreak);
        assert_eq!(t.remaining, Duration::from_secs(15 * 60));
        assert_eq!(
            t.cycle_position, 0,
            "cycle resets after long break is scheduled"
        );
    }

    #[test]
    fn skipping_work_does_not_count_pomodoro() {
        let mut t = timer();
        t.skip();
        assert_eq!(t.phase, TimerPhase::ShortBreak);
        assert_eq!(
            t.pomodoros_completed, 0,
            "a skipped work session is not a completed pomodoro"
        );
    }

    #[test]
    fn skipping_work_does_not_advance_cycle_toward_long_break() {
        let mut t = timer();
        for _ in 0..4 {
            t.skip(); // skip work
            t.skip(); // skip break
        }
        // Four skipped "cycles" later we must still be headed to a short break
        t.skip();
        assert_eq!(
            t.phase,
            TimerPhase::ShortBreak,
            "skips must not earn a long break"
        );
        assert_eq!(t.pomodoros_completed, 0);
    }

    #[test]
    fn break_returns_to_work_idle_by_default() {
        let mut t = timer();
        complete_phase(&mut t);
        t.advance_to_next_phase(); // -> ShortBreak
        complete_phase(&mut t);
        t.advance_to_next_phase(); // -> Work
        assert_eq!(t.phase, TimerPhase::Work);
        // auto_start_pomodoros defaults to false
        assert_eq!(t.status, TimerStatus::Idle);
    }

    #[test]
    fn reset_restores_full_duration_and_idle() {
        let mut t = timer();
        t.toggle_pause();
        t.tick();
        t.reset();
        assert_eq!(t.remaining, Duration::from_secs(25 * 60));
        assert_eq!(t.status, TimerStatus::Idle);
    }

    #[test]
    fn progress_goes_from_zero_to_one() {
        let mut t = timer();
        assert_eq!(t.progress(), 0.0);
        t.status = TimerStatus::Running;
        t.remaining = Duration::from_secs(25 * 60 / 2);
        assert!((t.progress() - 0.5).abs() < 1e-9);
        t.remaining = Duration::ZERO;
        assert_eq!(t.progress(), 1.0);
    }

    #[test]
    fn adjust_adds_a_minute_to_remaining_and_total() {
        let mut t = timer();
        t.adjust(1);
        assert_eq!(t.remaining, Duration::from_secs(26 * 60));
        assert_eq!(t.total_duration, Duration::from_secs(26 * 60));
    }

    #[test]
    fn adjust_subtracts_but_never_below_one_minute() {
        let mut t = timer();
        t.adjust(-1);
        assert_eq!(t.remaining, Duration::from_secs(24 * 60));

        t.remaining = Duration::from_secs(90);
        t.adjust(-1);
        assert_eq!(t.remaining, Duration::from_secs(60), "floor is one minute");

        t.adjust(-1);
        assert_eq!(t.remaining, Duration::from_secs(60), "already at the floor");
    }

    #[test]
    fn adjust_keeps_progress_in_bounds() {
        let mut t = timer();
        // Shrink remaining well below total, then grow it past total
        t.remaining = Duration::from_secs(120);
        t.adjust(1);
        t.adjust(1);
        assert!(
            t.total_duration >= t.remaining,
            "total must cover remaining"
        );
        assert!((0.0..=1.0).contains(&t.progress()));
    }

    #[test]
    fn last_minute_only_while_running() {
        let mut t = timer();
        t.remaining = Duration::from_secs(59);
        assert!(
            !t.is_last_minute(),
            "idle timer is not in last-minute state"
        );
        t.status = TimerStatus::Running;
        assert!(t.is_last_minute());
        t.remaining = Duration::from_secs(61);
        assert!(!t.is_last_minute());
    }
}
