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
pub struct Storage {
    /// Path to the sessions JSON file.
    data_path: PathBuf,
}

impl Storage {
    /// Create a new Storage pointing to `~/.local/share/pomodomate/sessions.json`.
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .context("Could not determine data directory")?
            .join("pomodomate");
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data dir {}", data_dir.display()))?;

        Ok(Self {
            data_path: data_dir.join("sessions.json"),
        })
    }

    /// Save a completed session to the history file.
    pub fn save_session(&self, session: &Session) -> Result<()> {
        let mut sessions = self.load_sessions().unwrap_or_default();
        sessions.push(session.clone());

        let json = serde_json::to_string_pretty(&sessions)
            .context("Failed to serialize sessions")?;
        fs::write(&self.data_path, json)
            .with_context(|| format!("Failed to write sessions to {}", self.data_path.display()))?;

        Ok(())
    }

    /// Load all sessions from disk.
    pub fn load_sessions(&self) -> Result<Vec<Session>> {
        if !self.data_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.data_path)
            .with_context(|| format!("Failed to read {}", self.data_path.display()))?;

        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let sessions: Vec<Session> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", self.data_path.display()))?;

        Ok(sessions)
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
