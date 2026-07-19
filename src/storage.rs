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

    /// Labels describing what this session was spent on.
    ///
    /// Defaults to empty so history written before tags existed still loads.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Seconds actually spent running this phase, excluding paused time.
    ///
    /// `duration_minutes` records what the phase was *meant* to last; this
    /// records what really happened, which is what a skipped or abandoned
    /// phase needs in order to be honest.
    #[serde(default)]
    pub focus_seconds: u64,
}

impl Session {
    /// Create a new session record.
    pub fn new(
        started_at: DateTime<Utc>,
        duration_minutes: u64,
        phase: &str,
        completed: bool,
        tags: Vec<String>,
        focus_seconds: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            started_at,
            ended_at: Utc::now(),
            duration_minutes,
            phase: phase.to_string(),
            completed,
            synced: false,
            tags,
            focus_seconds,
        }
    }

    /// Whether this session carries the given tag (case-insensitive).
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }
}

/// Normalize a comma-separated tag list into trimmed, non-empty, deduplicated
/// lowercase tags. Used for both `--tag` flags and config values.
pub fn parse_tags(raw: &[String]) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for entry in raw {
        for tag in entry.split(',') {
            let tag = tag.trim().to_lowercase();
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }
    }
    tags
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
            // A torn write or hand-edit damages one line; the whole history
            // must stay readable. Skipping beats failing, because the
            // alternative is a user who can never see their stats again.
            match serde_json::from_str(line) {
                Ok(session) => sessions.push(session),
                Err(_) => continue,
            }
        }

        Ok(sessions)
    }

    /// Number of unreadable lines in the history file.
    ///
    /// Lets callers surface "3 damaged records were skipped" instead of
    /// silently under-reporting.
    pub fn damaged_lines(&self) -> Result<usize> {
        if !self.data_path.exists() {
            return Ok(0);
        }
        let content = fs::read_to_string(&self.data_path)
            .with_context(|| format!("Failed to read {}", self.data_path.display()))?;

        Ok(content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| serde_json::from_str::<Session>(line).is_err())
            .count())
    }

    /// One-time migration from the old JSON-array format to JSON Lines.
    /// The old file is kept as `sessions.json.bak` after a successful migration.
    fn migrate_legacy(&self) -> Result<()> {
        if !self.legacy_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.legacy_path)
            .with_context(|| format!("Failed to read {}", self.legacy_path.display()))?;

        let legacy: Vec<Session> = if content.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse {}", self.legacy_path.display()))?
        };

        // Anything already in the JSONL file, so a migration interrupted
        // before the legacy file was renamed can be replayed without
        // duplicating records.
        let existing = self.read_jsonl()?;
        let already_migrated: std::collections::HashSet<&str> =
            existing.iter().map(|s| s.id.as_str()).collect();

        let mut lines = String::new();
        // Legacy sessions predate everything appended to the JSONL file, so
        // they go first.
        for session in legacy
            .iter()
            .filter(|s| !already_migrated.contains(s.id.as_str()))
        {
            lines.push_str(&serde_json::to_string(session).context("Failed to serialize session")?);
            lines.push('\n');
        }
        if self.data_path.exists() {
            lines.push_str(&fs::read_to_string(&self.data_path)?);
        }

        // Write through a temporary file and rename into place: a truncating
        // write would lose the whole history if the process died mid-write.
        let temp_path = self.data_path.with_extension("jsonl.tmp");
        fs::write(&temp_path, lines)
            .with_context(|| format!("Failed to write {}", temp_path.display()))?;
        fs::rename(&temp_path, &self.data_path)
            .with_context(|| format!("Failed to replace {}", self.data_path.display()))?;

        let backup = self.legacy_path.with_extension("json.bak");
        fs::rename(&self.legacy_path, &backup)
            .with_context(|| format!("Failed to back up {}", self.legacy_path.display()))?;

        Ok(())
    }

    /// Parse the JSONL file without attempting a migration first.
    ///
    /// Separate from [`Storage::load_sessions`] so the migration can inspect
    /// current contents without recursing into itself.
    fn read_jsonl(&self) -> Result<Vec<Session>> {
        if !self.data_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.data_path)
            .with_context(|| format!("Failed to read {}", self.data_path.display()))?;

        Ok(content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
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
        self.daily_counts_tagged(days, None)
    }

    /// Same as [`Storage::daily_counts`], but counting only sessions carrying
    /// `tag` when one is given.
    ///
    /// Days are the user's *local* calendar days. Sessions are stored as UTC
    /// instants, which is right, but grouping them by UTC date would move the
    /// day boundary away from local midnight — in UTC-5 the day would roll
    /// over at 19:00, splitting an evening's work across two days and
    /// breaking streaks.
    pub fn daily_counts_tagged(
        &self,
        days: u32,
        tag: Option<&str>,
    ) -> Result<Vec<(chrono::NaiveDate, u32)>> {
        let now = Utc::now();
        // Reach back an extra day: a local day can start up to 14 hours
        // before the same UTC day.
        let from = now - chrono::Duration::days(days as i64 + 1);
        let sessions = self.get_sessions_in_range(from, now)?;

        let mut counts: std::collections::HashMap<chrono::NaiveDate, u32> =
            std::collections::HashMap::new();

        for session in sessions {
            let matches_tag = tag.is_none_or(|t| session.has_tag(t));
            if session.phase == "work" && session.completed && matches_tag {
                let date = local_date(session.started_at);
                *counts.entry(date).or_insert(0) += 1;
            }
        }

        // Build a full list of days with counts. Stepping back over calendar
        // dates rather than subtracting 24-hour spans keeps the sequence
        // correct across daylight-saving changes, where a local day can be 23
        // or 25 hours long.
        let today = local_date(now);
        let mut result = Vec::new();
        for i in 0..days {
            let Some(date) = today.checked_sub_days(chrono::Days::new(i as u64)) else {
                break;
            };
            let count = counts.get(&date).copied().unwrap_or(0);
            result.push((date, count));
        }
        result.reverse();

        Ok(result)
    }
}

/// The local calendar date an instant falls on.
///
/// Sessions are stored as UTC instants — the correct way to record a moment —
/// but "which day did I work on" is a question about the user's wall clock.
pub fn local_date(instant: DateTime<Utc>) -> chrono::NaiveDate {
    instant.with_timezone(&chrono::Local).date_naive()
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
    /// Completed work pomodoros per tag over the last 365 days, ordered from
    /// most to least used. Sessions without tags are not counted.
    pub fn tag_totals(&self) -> Result<Vec<(String, u32)>> {
        let now = Utc::now();
        let from = now - chrono::Duration::days(365);
        let sessions = self.get_sessions_in_range(from, now)?;

        let mut totals: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for session in sessions {
            if session.phase == "work" && session.completed {
                for tag in &session.tags {
                    *totals.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        let mut totals: Vec<(String, u32)> = totals.into_iter().collect();
        // Most used first, alphabetical within a tie so output is stable.
        totals.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Ok(totals)
    }

    /// Compute aggregate stats over the last 365 days, optionally restricted
    /// to sessions carrying `tag`.
    pub fn stats_tagged(&self, tag: Option<&str>) -> Result<Stats> {
        let daily = self.daily_counts_tagged(365, tag)?;

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
        Session::new(Utc::now(), 25, phase, completed, Vec::new(), 25 * 60)
    }

    fn tagged_session(phase: &str, completed: bool, tags: &[&str]) -> Session {
        let tags = tags.iter().map(|t| t.to_string()).collect();
        Session::new(Utc::now(), 25, phase, completed, tags, 25 * 60)
    }

    #[test]
    fn local_date_follows_the_system_timezone_not_utc() {
        use chrono::{Offset, TimeZone};

        // 02:30 UTC: still the previous day anywhere west of UTC-3.
        let instant = Utc.with_ymd_and_hms(2026, 7, 19, 2, 30, 0).unwrap();
        let offset = chrono::Local
            .offset_from_utc_datetime(&instant.naive_utc())
            .fix()
            .local_minus_utc();

        let expected = (instant + chrono::Duration::seconds(offset as i64)).date_naive();
        assert_eq!(local_date(instant), expected);

        if offset <= -3 * 3600 {
            assert_ne!(
                local_date(instant),
                instant.date_naive(),
                "west of UTC-3, 02:30 UTC belongs to the previous local day"
            );
        }
    }

    #[test]
    fn a_session_recorded_now_counts_towards_today() {
        let (storage, _dir) = temp_storage();
        storage.save_session(&session("work", true)).unwrap();

        let counts = storage.daily_counts(7).unwrap();
        let (last_day, count) = counts.last().copied().unwrap();

        assert_eq!(
            last_day,
            chrono::Local::now().date_naive(),
            "the newest bucket must be the user's local today"
        );
        assert_eq!(count, 1, "a pomodoro finished now belongs to today");
    }

    #[test]
    fn daily_counts_walks_back_over_calendar_days() {
        let (storage, _dir) = temp_storage();
        let counts = storage.daily_counts(5).unwrap();

        assert_eq!(counts.len(), 5);
        // Consecutive, ascending, exactly one calendar day apart — the
        // property that subtracting 24-hour spans breaks across DST.
        for pair in counts.windows(2) {
            assert_eq!(
                pair[1].0.signed_duration_since(pair[0].0).num_days(),
                1,
                "days must be consecutive: {:?} then {:?}",
                pair[0].0,
                pair[1].0
            );
        }
    }

    #[test]
    fn a_damaged_line_does_not_hide_the_rest_of_the_history() {
        let (storage, dir) = temp_storage();
        let good = serde_json::to_string(&session("work", true)).unwrap();
        let also_good = serde_json::to_string(&session("work", true)).unwrap();
        // Middle line truncated, as a torn write would leave it.
        let torn = &good[..good.len() / 2];
        fs::write(
            dir.join("sessions.jsonl"),
            format!("{good}\n{torn}\n{also_good}\n"),
        )
        .unwrap();

        let loaded = storage.load_sessions().unwrap();
        assert_eq!(loaded.len(), 2, "readable sessions must survive");
        assert_eq!(storage.damaged_lines().unwrap(), 1);
    }

    #[test]
    fn migration_replayed_after_a_crash_does_not_duplicate() {
        let dir = std::env::temp_dir()
            .join("pomodomate-test")
            .join(Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).unwrap();

        let legacy = vec![session("work", true), session("work", true)];
        let legacy_json = serde_json::to_string_pretty(&legacy).unwrap();
        fs::write(dir.join("sessions.json"), &legacy_json).unwrap();

        let storage = Storage::at_dir(dir.clone());
        assert_eq!(storage.load_sessions().unwrap().len(), 2);

        // Simulate dying after the history was rewritten but before the
        // legacy file was renamed: put it back and migrate again.
        fs::write(dir.join("sessions.json"), &legacy_json).unwrap();
        let loaded = storage.load_sessions().unwrap();

        assert_eq!(
            loaded.len(),
            2,
            "replaying an interrupted migration must not duplicate records"
        );
    }

    #[test]
    fn parse_tags_trims_lowercases_and_deduplicates() {
        let raw = vec!["Tesis, rust".to_string(), " RUST ".to_string()];
        assert_eq!(parse_tags(&raw), vec!["tesis", "rust"]);
    }

    #[test]
    fn parse_tags_drops_empty_entries() {
        let raw = vec![" , ,".to_string(), String::new()];
        assert!(parse_tags(&raw).is_empty());
    }

    #[test]
    fn has_tag_ignores_case() {
        let s = tagged_session("work", true, &["tesis"]);
        assert!(s.has_tag("TESIS"));
        assert!(!s.has_tag("rust"));
    }

    #[test]
    fn sessions_written_before_tags_existed_still_load() {
        let (storage, dir) = temp_storage();
        // A history line from v0.2.0, with no `tags` field at all.
        let legacy_line = r#"{"id":"abc","started_at":"2026-01-01T10:00:00Z","ended_at":"2026-01-01T10:25:00Z","duration_minutes":25,"phase":"work","completed":true,"synced":false}"#;
        fs::write(dir.join("sessions.jsonl"), format!("{legacy_line}\n")).unwrap();

        let loaded = storage.load_sessions().unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(
            loaded[0].tags.is_empty(),
            "missing tags must default to empty, not fail the parse"
        );
    }

    #[test]
    fn stats_can_be_restricted_to_a_tag() {
        let (storage, _dir) = temp_storage();
        storage
            .save_session(&tagged_session("work", true, &["tesis"]))
            .unwrap();
        storage
            .save_session(&tagged_session("work", true, &["tesis", "rust"]))
            .unwrap();
        storage
            .save_session(&tagged_session("work", true, &["rust"]))
            .unwrap();
        storage.save_session(&session("work", true)).unwrap(); // untagged

        assert_eq!(storage.stats_tagged(None).unwrap().today, 4);
        assert_eq!(storage.stats_tagged(Some("tesis")).unwrap().today, 2);
        assert_eq!(storage.stats_tagged(Some("rust")).unwrap().today, 2);
        assert_eq!(storage.stats_tagged(Some("ocio")).unwrap().today, 0);
    }

    #[test]
    fn tag_totals_rank_by_use_and_ignore_uncounted_sessions() {
        let (storage, _dir) = temp_storage();
        for _ in 0..3 {
            storage
                .save_session(&tagged_session("work", true, &["rust"]))
                .unwrap();
        }
        storage
            .save_session(&tagged_session("work", true, &["tesis"]))
            .unwrap();
        // Neither of these should count toward any tag.
        storage
            .save_session(&tagged_session("work", false, &["tesis"]))
            .unwrap();
        storage
            .save_session(&tagged_session("short_break", true, &["tesis"]))
            .unwrap();

        let totals = storage.tag_totals().unwrap();
        assert_eq!(
            totals,
            vec![("rust".to_string(), 3), ("tesis".to_string(), 1)]
        );
    }

    #[test]
    fn tag_totals_are_empty_without_tagged_sessions() {
        let (storage, _dir) = temp_storage();
        storage.save_session(&session("work", true)).unwrap();
        assert!(storage.tag_totals().unwrap().is_empty());
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
        Session::new(
            Utc::now() - chrono::Duration::days(days),
            25,
            "work",
            true,
            Vec::new(),
            25 * 60,
        )
    }

    #[test]
    fn stats_aggregates_today_week_and_year() {
        let (storage, _dir) = temp_storage();
        storage.save_session(&work_session_days_ago(0)).unwrap();
        storage.save_session(&work_session_days_ago(0)).unwrap();
        storage.save_session(&work_session_days_ago(3)).unwrap();
        storage.save_session(&work_session_days_ago(10)).unwrap();
        storage.save_session(&work_session_days_ago(400)).unwrap(); // outside the year window

        let stats = storage.stats_tagged(None).unwrap();
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

        let stats = storage.stats_tagged(None).unwrap();
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

        let stats = storage.stats_tagged(None).unwrap();
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
        // Local, not UTC: past 19:00 in UTC-5 the UTC date is already
        // tomorrow and would not appear in the user's list of days at all.
        let today = chrono::Local::now().date_naive();
        let today_count = counts.iter().find(|(d, _)| *d == today).unwrap().1;
        assert_eq!(today_count, 2);
    }
}
