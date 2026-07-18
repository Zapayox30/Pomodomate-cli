use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::hooks;
use crate::storage::{Session, Storage};
use crate::timer::{Timer, TimerPhase, TimerStatus};

/// Everything that happens to a pomodoro regardless of who is driving it.
///
/// The TUI and the daemon both own an `Engine`: it holds the timer, records
/// sessions, fires hooks and raises notifications, so those rules live in one
/// place instead of being reimplemented per front-end.
pub struct Engine {
    /// The timer state machine.
    pub timer: Timer,
    /// User configuration for this run.
    pub config: Config,
    /// Local session history.
    pub storage: Storage,
    /// True while no work session has been completed today.
    pub first_session_today: bool,
    /// True when the timer was paused because the user stepped away.
    ///
    /// Stays set after they come back, so the front-end can explain why the
    /// clock is stopped instead of silently resuming.
    pub paused_by_idle: bool,
    /// When the current phase started, if it is being timed.
    phase_started_at: Option<DateTime<Utc>>,
    /// Looping background track, active only during running work phases.
    ambient: crate::sound::Ambient,
}

/// A point-in-time view of the engine, safe to serialize and send over IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    /// `work`, `short_break` or `long_break`.
    pub phase: String,
    /// `running`, `paused`, `idle` or `completed`.
    pub status: String,
    /// Seconds left in the current phase.
    pub remaining_seconds: u64,
    /// Full length of the current phase, in seconds.
    pub total_seconds: u64,
    /// Progress through the phase, 0-100.
    pub percent: u8,
    /// Work pomodoros completed since the timer started.
    pub pomodoros: u32,
    /// Remaining time preformatted as `MM:SS`.
    pub time: String,
    /// True when the pause was caused by the user stepping away.
    #[serde(default)]
    pub idle_paused: bool,
}

impl Status {
    /// A phase-appropriate emoji, handy for status bars.
    pub fn icon(&self) -> &'static str {
        match (self.phase.as_str(), self.status.as_str()) {
            (_, "paused") => "⏸",
            ("work", _) => "🍅",
            ("short_break", _) => "☕",
            ("long_break", _) => "🌴",
            _ => "🍅",
        }
    }

    /// Expand a user template such as `"{icon} {time}"`.
    ///
    /// Unknown placeholders are left untouched so a typo shows up in the bar
    /// instead of silently disappearing.
    pub fn render(&self, template: &str) -> String {
        template
            .replace("{icon}", self.icon())
            .replace("{time}", &self.time)
            .replace("{percent}", &self.percent.to_string())
            .replace("{phase}", &self.phase)
            .replace("{status}", &self.status)
            .replace("{pomodoros}", &self.pomodoros.to_string())
            .replace("{remaining}", &self.remaining_seconds.to_string())
            .replace("{total}", &self.total_seconds.to_string())
    }
}

impl Engine {
    /// Build an engine backed by the user's real history file.
    pub fn new(config: Config) -> Result<Self> {
        let storage = Storage::new()?;
        Ok(Self::with_storage(config, storage))
    }

    /// Build an engine over a specific storage, used by tests.
    pub fn with_storage(config: Config, storage: Storage) -> Self {
        let timer = Timer::new(&config);
        let first_session_today = storage
            .daily_counts(1)
            .map(|counts| counts.iter().all(|(_, n)| *n == 0))
            .unwrap_or(true);

        Self {
            timer,
            config,
            storage,
            first_session_today,
            paused_by_idle: false,
            phase_started_at: None,
            ambient: crate::sound::Ambient::new(),
        }
    }

    /// Start or stop the ambient track so it matches the timer.
    ///
    /// Called after every state change rather than on a timer, and it only
    /// acts on transitions, so a running track is never restarted.
    fn sync_ambient(&mut self) {
        let should_play = self.timer.phase == TimerPhase::Work
            && self.timer.status == TimerStatus::Running
            && !self.config.ambient_sound.is_empty();

        if should_play {
            if !self.ambient.is_playing() {
                let dir = crate::sound::sounds_dir();
                if let Some(path) =
                    crate::sound::resolve(&self.config.ambient_sound, dir.as_deref())
                {
                    self.ambient.play(&path);
                }
            }
        } else if self.ambient.is_playing() {
            self.ambient.stop();
        }
    }

    // ── Controls ─────────────────────────────────────────────────────────

    /// Start or pause, mirroring the space bar. Starting an idle timer begins
    /// timing the phase and fires its start hook.
    pub fn toggle(&mut self) {
        let starting = self.timer.status == TimerStatus::Idle;
        if starting {
            self.phase_started_at = Some(Utc::now());
        }
        // Any deliberate control clears the away notice.
        self.paused_by_idle = false;
        self.timer.toggle_pause();
        if starting {
            self.fire_phase_start();
        }
        self.sync_ambient();
    }

    /// Pause because the user stepped away from the desk.
    ///
    /// Only a running timer is affected, and the reason is remembered so the
    /// front-end can tell them what happened when they return.
    pub fn pause_for_idle(&mut self) {
        if self.timer.status == TimerStatus::Running {
            self.timer.status = TimerStatus::Paused;
            self.paused_by_idle = true;
            self.sync_ambient();
        }
    }

    /// Pause a running timer. Idempotent: pausing an already paused or idle
    /// timer does nothing, so scripts can call it blindly.
    pub fn pause(&mut self) {
        if self.timer.status == TimerStatus::Running {
            self.timer.status = TimerStatus::Paused;
            self.paused_by_idle = false;
            self.sync_ambient();
        }
    }

    /// Resume a paused timer, or start an idle one. Idempotent.
    pub fn resume(&mut self) {
        self.paused_by_idle = false;
        match self.timer.status {
            TimerStatus::Paused => self.timer.status = TimerStatus::Running,
            TimerStatus::Idle => self.toggle(),
            _ => {}
        }
        self.sync_ambient();
    }

    /// Reset the current phase back to its full duration.
    pub fn reset(&mut self) {
        self.timer.reset();
        self.paused_by_idle = false;
        self.phase_started_at = None;
        self.sync_ambient();
    }

    /// Abandon the current phase and move to the next one.
    pub fn skip(&mut self) -> Result<()> {
        if let Some(started) = self.phase_started_at.take() {
            self.save_session(started, false)?;
        }
        // The phase ends here even though it was not completed, so teardown
        // hooks still get their turn.
        self.fire_phase_end(false);
        self.timer.skip();
        if self.timer.status == TimerStatus::Running {
            self.phase_started_at = Some(Utc::now());
            self.fire_phase_start();
        }
        self.sync_ambient();
        Ok(())
    }

    /// Add or remove whole minutes from the running phase.
    pub fn adjust(&mut self, minutes: i64) {
        self.timer.adjust(minutes);
    }

    /// Advance one second. Returns `true` when a phase completed, after all of
    /// its side effects (session, notification, hooks, transition) have run.
    pub fn tick(&mut self) -> Result<bool> {
        if !self.timer.tick() {
            return Ok(false);
        }
        self.on_phase_complete()?;
        Ok(true)
    }

    /// Everything that must happen when a phase runs out.
    fn on_phase_complete(&mut self) -> Result<()> {
        if let Some(started) = self.phase_started_at.take() {
            self.save_session(started, true)?;
        }

        if self.timer.phase == TimerPhase::Work {
            self.first_session_today = false;
        }

        if self.config.notifications {
            self.send_notification();
        }

        if self.config.sound {
            ring_bell();
        }

        // Fire the end hook while `timer.phase` still names the phase that
        // just finished.
        self.fire_phase_end(true);

        self.timer.advance_to_next_phase();

        if self.timer.status == TimerStatus::Running {
            self.phase_started_at = Some(Utc::now());
            self.fire_phase_start();
        }

        self.sync_ambient();
        Ok(())
    }

    // ── Reporting ────────────────────────────────────────────────────────

    /// Snapshot the current state for display or IPC.
    pub fn status(&self) -> Status {
        Status {
            phase: phase_name(self.timer.phase).to_string(),
            status: match self.timer.status {
                TimerStatus::Running => "running",
                TimerStatus::Paused => "paused",
                TimerStatus::Idle => "idle",
                TimerStatus::Completed => "completed",
            }
            .to_string(),
            remaining_seconds: self.timer.remaining.as_secs(),
            total_seconds: self.timer.total_duration.as_secs(),
            percent: (self.timer.progress() * 100.0).round() as u8,
            pomodoros: self.timer.pomodoros_completed,
            time: self.timer.remaining_display(),
            idle_paused: self.paused_by_idle,
        }
    }

    /// Apply an inactivity signal from the compositor.
    ///
    /// Returning is deliberately not the same as resuming: the timer stays
    /// paused until the user says otherwise, so time spent away is never
    /// counted as focus.
    pub fn handle_idle(&mut self, event: crate::idle::IdleEvent) {
        if event == crate::idle::IdleEvent::Idled {
            self.pause_for_idle();
        }
    }

    // ── Internals ────────────────────────────────────────────────────────

    /// Configured length of a phase in minutes.
    fn phase_duration(&self, phase: TimerPhase) -> u64 {
        match phase {
            TimerPhase::Work => self.config.work_duration,
            TimerPhase::ShortBreak => self.config.short_break,
            TimerPhase::LongBreak => self.config.long_break,
        }
    }

    /// Append a session record to local history.
    fn save_session(&self, started: DateTime<Utc>, completed: bool) -> Result<()> {
        let session = Session::new(
            started,
            self.phase_duration(self.timer.phase),
            phase_name(self.timer.phase),
            completed,
            self.config.tags.clone(),
        );
        self.storage.save_session(&session)
    }

    /// Describe the current phase for a hook invocation.
    fn hook_context(&self, completed: bool) -> hooks::HookContext {
        hooks::HookContext {
            phase: phase_name(self.timer.phase).to_string(),
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
            hooks::spawn(line, &self.hook_context(false));
        }
    }

    /// Fire the end hook for the phase that is finishing.
    fn fire_phase_end(&self, completed: bool) {
        let hook = match self.timer.phase {
            TimerPhase::Work => &self.config.hooks.work_end,
            TimerPhase::ShortBreak | TimerPhase::LongBreak => &self.config.hooks.break_end,
        };
        if let Some(line) = hook {
            hooks::spawn(line, &self.hook_context(completed));
        }
    }

    /// Send a desktop notification for the phase that just ended.
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
            // XDG sound theme name; honored by daemons with sound support.
            notification.sound_name("complete");
        }

        // Fire-and-forget: a missing notification daemon must not be fatal.
        let _ = notification.show();
    }
}

/// Stable identifier for a phase, used in stored sessions, hooks and IPC.
pub fn phase_name(phase: TimerPhase) -> &'static str {
    match phase {
        TimerPhase::Work => "work",
        TimerPhase::ShortBreak => "short_break",
        TimerPhase::LongBreak => "long_break",
    }
}

/// Ring the terminal bell. Works even when notifications are disabled.
fn ring_bell() {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(b"\x07");
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn engine() -> Engine {
        let dir = std::env::temp_dir()
            .join("pomodomate-engine-test")
            .join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        // Keep tests quiet and side-effect free.
        let config = Config {
            notifications: false,
            sound: false,
            ..Default::default()
        };
        Engine::with_storage(config, Storage::at_dir(dir))
    }

    /// Drive the current phase to its final second and tick it over.
    fn complete_phase(e: &mut Engine) {
        e.timer.status = TimerStatus::Running;
        e.timer.remaining = std::time::Duration::from_secs(1);
        assert!(e.tick().unwrap(), "phase should have completed");
    }

    #[test]
    fn toggle_starts_an_idle_timer() {
        let mut e = engine();
        assert_eq!(e.timer.status, TimerStatus::Idle);
        e.toggle();
        assert_eq!(e.timer.status, TimerStatus::Running);
    }

    #[test]
    fn completing_work_records_a_session() {
        let mut e = engine();
        e.toggle();
        complete_phase(&mut e);

        let sessions = e.storage.load_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].phase, "work");
        assert!(sessions[0].completed);
    }

    #[test]
    fn skipping_records_an_incomplete_session() {
        let mut e = engine();
        e.toggle();
        e.skip().unwrap();

        let sessions = e.storage.load_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(
            !sessions[0].completed,
            "a skipped phase is not a completed one"
        );
    }

    #[test]
    fn sessions_carry_the_configured_tags() {
        let mut e = engine();
        e.config.tags = vec!["tesis".to_string()];
        e.toggle();
        complete_phase(&mut e);

        let sessions = e.storage.load_sessions().unwrap();
        assert_eq!(sessions[0].tags, vec!["tesis".to_string()]);
    }

    #[test]
    fn resetting_discards_the_pending_session() {
        let mut e = engine();
        e.toggle();
        e.reset();
        e.skip().unwrap();

        assert!(
            e.storage.load_sessions().unwrap().is_empty(),
            "a reset phase was never timed, so nothing should be recorded"
        );
    }

    #[test]
    fn going_idle_pauses_a_running_timer() {
        let mut e = engine();
        e.toggle();
        e.handle_idle(crate::idle::IdleEvent::Idled);

        assert_eq!(e.status().status, "paused");
        assert!(e.paused_by_idle, "the pause reason should be remembered");
        assert!(e.status().idle_paused);
    }

    #[test]
    fn coming_back_does_not_resume_by_itself() {
        let mut e = engine();
        e.toggle();
        e.handle_idle(crate::idle::IdleEvent::Idled);
        e.handle_idle(crate::idle::IdleEvent::Resumed);

        assert_eq!(
            e.status().status,
            "paused",
            "time away must not be counted as focus"
        );
        assert!(
            e.paused_by_idle,
            "the notice stays until the user acts on it"
        );
    }

    #[test]
    fn resuming_by_hand_clears_the_idle_notice() {
        let mut e = engine();
        e.toggle();
        e.handle_idle(crate::idle::IdleEvent::Idled);
        e.resume();

        assert_eq!(e.status().status, "running");
        assert!(!e.paused_by_idle);
        assert!(!e.status().idle_paused);
    }

    #[test]
    fn idle_does_not_disturb_a_timer_that_was_never_started() {
        let mut e = engine();
        e.handle_idle(crate::idle::IdleEvent::Idled);

        assert_eq!(e.status().status, "idle");
        assert!(
            !e.paused_by_idle,
            "nothing was running, so nothing was paused"
        );
    }

    #[test]
    fn a_manual_pause_is_not_reported_as_an_idle_pause() {
        let mut e = engine();
        e.toggle();
        e.pause();

        assert_eq!(e.status().status, "paused");
        assert!(!e.status().idle_paused);
    }

    #[test]
    fn status_reports_progress_and_phase() {
        let mut e = engine();
        let s = e.status();
        assert_eq!(s.phase, "work");
        assert_eq!(s.status, "idle");
        assert_eq!(s.percent, 0);
        assert_eq!(s.time, "25:00");

        e.toggle();
        e.timer.remaining = std::time::Duration::from_secs(5 * 60);
        let s = e.status();
        assert_eq!(s.status, "running");
        assert_eq!(s.percent, 80);
        assert_eq!(s.time, "05:00");
    }

    #[test]
    fn status_template_expands_known_placeholders() {
        let e = engine();
        let s = e.status();
        assert_eq!(s.render("{icon} {time}"), "🍅 25:00");
        assert_eq!(s.render("{phase}:{percent}%"), "work:0%");
        assert_eq!(s.render("{pomodoros}"), "0");
    }

    #[test]
    fn status_template_leaves_unknown_placeholders_alone() {
        let e = engine();
        assert_eq!(e.status().render("{nope}"), "{nope}");
    }

    #[test]
    fn paused_status_uses_the_pause_icon() {
        let mut e = engine();
        e.toggle();
        e.toggle();
        assert_eq!(e.status().status, "paused");
        assert_eq!(e.status().icon(), "⏸");
    }
}
