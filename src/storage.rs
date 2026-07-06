use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// A single Pomodoro session record.
///
/// Stored in `~/.local/share/pomodomate/sessions.json`.
/// Format is designed to be compatible with the Pomodomate API (Phase 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier (UUIDv4)
    pub id: String,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// When the session ended
    pub ended_at: DateTime<Utc>,
    /// Intended duration in minutes
    pub duration_minutes: u64,
    /// Phase type: "work", "short_break", or "long_break"
    pub phase: String,
    /// Whether the full duration was completed (not skipped/reset)
    pub completed: bool,
    /// Whether this session has been synced to the API (Phase 2)
    #[serde(default)]
    pub synced: bool,
}

impl Session {
    /// Create a new session record.
    pub fn new(
        started_at: DateTime<Utc>,
        duration_minutes: u64,
        phase: &str,
        completed: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            started_at,
            ended_at: Utc::now(),
            duration_minutes,
            phase: phase.to_string(),
            completed,
            synced: false,
        }
    }
}

/// Manages local session history on disk.
///
/// History is stored as JSON Lines (one session per line) so saving is an
/// append instead of a full rewrite, and a partial write can only affect the
/// last line rather than corrupting the whole file.
pub struct Storage {
    /// Path to the append-only JSON Lines history file.
    data_path: PathBuf,
    /// Legacy JSON-array file (old format), migrated on first access.
    legacy_path: PathBuf,
}

impl Storage {
    /// Create a new Storage pointing to `~/.local/share/pomodomate/sessions.jsonl`.
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .context("Could not determine data directory")?
            .join("pomodomate");
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data dir {}", data_dir.display()))?;

        Ok(Self::at_dir(data_dir))
    }

    /// Create a Storage rooted at a specific directory (must already exist).
    pub fn at_dir(dir: PathBuf) -> Self {
        Self {
            data_path: dir.join("sessions.jsonl"),
            legacy_path: dir.join("sessions.json"),
        }
    }

    /// Append a session to the history file.
    pub fn save_session(&self, session: &Session) -> Result<()> {
        self.migrate_legacy()?;

        let mut line = serde_json::to_string(session).context("Failed to serialize session")?;
        line.push('\n');

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.data_path)
            .with_context(|| format!("Failed to open {}", self.data_path.display()))?;
        use std::io::Write;
        file.write_all(line.as_bytes())
            .with_context(|| format!("Failed to write to {}", self.data_path.display()))?;

        Ok(())
    }

    /// Load all sessions from disk.
    pub fn load_sessions(&self) -> Result<Vec<Session>> {
        self.migrate_legacy()?;

        if !self.data_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.data_path)
            .with_context(|| format!("Failed to read {}", self.data_path.display()))?;

        let mut sessions = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let session: Session = serde_json::from_str(line).with_context(|| {
                format!("Failed to parse a session in {}", self.data_path.display())
            })?;
            sessions.push(session);
        }

        Ok(sessions)
    }

    /// One-time migration from the old JSON-array format to JSON Lines.
    /// The old file is kept as `sessions.json.bak` after a successful migration.
    fn migrate_legacy(&self) -> Result<()> {
        if !self.legacy_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.legacy_path)
            .with_context(|| format!("Failed to read {}", self.legacy_path.display()))?;

        let mut lines = String::new();
        if !content.trim().is_empty() {
            let legacy: Vec<Session> = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse {}", self.legacy_path.display()))?;
            for session in &legacy {
                lines.push_str(
                    &serde_json::to_string(session).context("Failed to serialize session")?,
                );
                lines.push('\n');
            }
        }

        // Legacy sessions are older than anything already appended to the
        // JSONL file, so they go first.
        if self.data_path.exists() {
            lines.push_str(&fs::read_to_string(&self.data_path)?);
        }
        fs::write(&self.data_path, lines)
            .with_context(|| format!("Failed to write {}", self.data_path.display()))?;

        let backup = self.legacy_path.with_extension("json.bak");
        fs::rename(&self.legacy_path, &backup)
            .with_context(|| format!("Failed to back up {}", self.legacy_path.display()))?;

        Ok(())
    }

    /// Get sessions within a date range (for heatmap).
    pub fn get_sessions_in_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Session>> {
        let sessions = self.load_sessions()?;
        Ok(sessions
            .into_iter()
            .filter(|s| s.started_at >= from && s.started_at <= to)
            .collect())
    }

    /// Count completed work sessions per day for the last N days (for heatmap).
    pub fn daily_counts(&self, days: u32) -> Result<Vec<(chrono::NaiveDate, u32)>> {
        let now = Utc::now();
        let from = now - chrono::Duration::days(days as i64);
        let sessions = self.get_sessions_in_range(from, now)?;

        let mut counts: std::collections::HashMap<chrono::NaiveDate, u32> =
            std::collections::HashMap::new();

        for session in sessions {
            if session.phase == "work" && session.completed {
                let date = session.started_at.date_naive();
                *counts.entry(date).or_insert(0) += 1;
            }
        }

        // Build a full list of days with counts
        let mut result = Vec::new();
        for i in 0..days {
            let date = (now - chrono::Duration::days(i as i64)).date_naive();
            let count = counts.get(&date).copied().unwrap_or(0);
            result.push((date, count));
        }
        result.reverse();

        Ok(result)
    }
}

/// Aggregated productivity stats (used by `pomodomate stats` and the TUI).
#[derive(Debug, Serialize)]
pub struct Stats {
    /// Completed pomodoros today
    pub today: u32,
    /// Completed pomodoros in the last 7 days (including today)
    pub week: u32,
    /// Completed pomodoros in the last 365 days
    pub year: u32,
    /// Days with at least one completed pomodoro in the last 365 days
    pub active_days: u32,
    /// Longest run of consecutive active days
    pub best_streak: u32,
    /// Consecutive active days ending today (or yesterday, if today has
    /// no pomodoros yet — the streak is still alive until the day ends)
    pub current_streak: u32,
}

impl Storage {
    /// Compute aggregate stats over the last 365 days.
    pub fn stats(&self) -> Result<Stats> {
        let daily = self.daily_counts(365)?;

        let today = daily.last().map(|(_, c)| *c).unwrap_or(0);
        let week = daily.iter().rev().take(7).map(|(_, c)| c).sum();
        let year = daily.iter().map(|(_, c)| c).sum();
        let active_days = daily.iter().filter(|(_, c)| *c > 0).count() as u32;

        Ok(Stats {
            today,
            week,
            year,
            active_days,
            best_streak: calculate_streak(&daily),
            current_streak: current_streak(&daily),
        })
    }
}

/// Longest run of consecutive days with at least one completed pomodoro.
pub fn calculate_streak(daily_counts: &[(chrono::NaiveDate, u32)]) -> u32 {
    let mut max_streak = 0u32;
    let mut current = 0u32;

    for (_date, count) in daily_counts {
        if *count > 0 {
            current += 1;
            max_streak = max_streak.max(current);
        } else {
            current = 0;
        }
    }

    max_streak
}

/// Consecutive active days ending today; an inactive today doesn't break
/// the streak (there's still time to keep it alive).
fn current_streak(daily_counts: &[(chrono::NaiveDate, u32)]) -> u32 {
    let mut iter = daily_counts.iter().rev().peekable();

    // Skip today if it has no pomodoros yet
    if let Some(&&(_, 0)) = iter.peek() {
        iter.next();
    }

    let mut streak = 0u32;
    for (_date, count) in iter {
        if *count > 0 {
            streak += 1;
        } else {
            break;
        }
    }
    streak
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh Storage rooted in a unique temp directory.
    fn temp_storage() -> (Storage, PathBuf) {
        let dir = std::env::temp_dir()
            .join("pomodomate-test")
            .join(Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).unwrap();
        (Storage::at_dir(dir.clone()), dir)
    }

    fn session(phase: &str, completed: bool) -> Session {
        Session::new(Utc::now(), 25, phase, completed)
    }

    #[test]
    fn load_returns_empty_when_no_history_exists() {
        let (storage, _dir) = temp_storage();
        assert!(storage.load_sessions().unwrap().is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let (storage, _dir) = temp_storage();
        storage.save_session(&session("work", true)).unwrap();
        storage
            .save_session(&session("short_break", false))
            .unwrap();

        let loaded = storage.load_sessions().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].phase, "work");
        assert!(loaded[0].completed);
        assert_eq!(loaded[1].phase, "short_break");
        assert!(!loaded[1].completed);
    }

    #[test]
    fn migrates_legacy_sessions_json() {
        let dir = std::env::temp_dir()
            .join("pomodomate-test")
            .join(Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).unwrap();

        // Write a legacy JSON-array history file
        let legacy = vec![session("work", true), session("work", true)];
        fs::write(
            dir.join("sessions.json"),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();

        let storage = Storage::at_dir(dir.clone());
        let loaded = storage.load_sessions().unwrap();
        assert_eq!(loaded.len(), 2, "legacy sessions must survive migration");

        // Appending after migration keeps everything
        storage.save_session(&session("work", false)).unwrap();
        assert_eq!(storage.load_sessions().unwrap().len(), 3);

        // The legacy file is no longer the source of truth
        assert!(!dir.join("sessions.json").exists());
    }

    fn work_session_days_ago(days: i64) -> Session {
        Session::new(Utc::now() - chrono::Duration::days(days), 25, "work", true)
    }

    #[test]
    fn stats_aggregates_today_week_and_year() {
        let (storage, _dir) = temp_storage();
        storage.save_session(&work_session_days_ago(0)).unwrap();
        storage.save_session(&work_session_days_ago(0)).unwrap();
        storage.save_session(&work_session_days_ago(3)).unwrap();
        storage.save_session(&work_session_days_ago(10)).unwrap();
        storage.save_session(&work_session_days_ago(400)).unwrap(); // outside the year window

        let stats = storage.stats().unwrap();
        assert_eq!(stats.today, 2);
        assert_eq!(stats.week, 3, "last 7 days include today and 3 days ago");
        assert_eq!(stats.year, 4);
        assert_eq!(stats.active_days, 3);
    }

    #[test]
    fn stats_streaks_count_consecutive_days() {
        let (storage, _dir) = temp_storage();
        // Active today, yesterday, and the day before: current streak of 3
        for days in 0..3 {
            storage.save_session(&work_session_days_ago(days)).unwrap();
        }
        // An older, longer streak of 4 separated by a gap
        for days in 10..14 {
            storage.save_session(&work_session_days_ago(days)).unwrap();
        }

        let stats = storage.stats().unwrap();
        assert_eq!(stats.current_streak, 3);
        assert_eq!(stats.best_streak, 4);
    }

    #[test]
    fn current_streak_survives_a_day_without_pomodoros_yet() {
        let (storage, _dir) = temp_storage();
        // Nothing today, but active yesterday and the day before:
        // the streak is still alive until today ends.
        storage.save_session(&work_session_days_ago(1)).unwrap();
        storage.save_session(&work_session_days_ago(2)).unwrap();

        let stats = storage.stats().unwrap();
        assert_eq!(stats.current_streak, 2);
    }

    #[test]
    fn daily_counts_only_counts_completed_work() {
        let (storage, _dir) = temp_storage();
        storage.save_session(&session("work", true)).unwrap();
        storage.save_session(&session("work", true)).unwrap();
        storage.save_session(&session("work", false)).unwrap(); // skipped
        storage.save_session(&session("short_break", true)).unwrap(); // break

        let counts = storage.daily_counts(1).unwrap();
        let today = Utc::now().date_naive();
        let today_count = counts.iter().find(|(d, _)| *d == today).unwrap().1;
        assert_eq!(today_count, 2);
    }
}
