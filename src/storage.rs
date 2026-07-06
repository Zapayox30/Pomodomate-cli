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
            let session: Session = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse a session in {}", self.data_path.display()))?;
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
                lines.push_str(&serde_json::to_string(session).context("Failed to serialize session")?);
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
        storage.save_session(&session("short_break", false)).unwrap();

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
