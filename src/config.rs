use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User-configurable settings for Pomodomate.
///
/// Stored in `~/.config/pomodomate/config.toml`.
/// All durations are in minutes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Duration of a work session in minutes (default: 25)
    #[serde(default = "default_work_duration")]
    pub work_duration: u64,

    /// Duration of a short break in minutes (default: 5)
    #[serde(default = "default_short_break")]
    pub short_break: u64,

    /// Duration of a long break in minutes (default: 15)
    #[serde(default = "default_long_break")]
    pub long_break: u64,

    /// Number of pomodoros before a long break (default: 4)
    #[serde(default = "default_long_break_interval")]
    pub long_break_interval: u32,

    /// Automatically start break timers (default: true)
    #[serde(default = "default_true")]
    pub auto_start_breaks: bool,

    /// Automatically start work timers after breaks (default: false)
    #[serde(default)]
    pub auto_start_pomodoros: bool,

    /// Enable desktop notifications (default: true)
    #[serde(default = "default_true")]
    pub notifications: bool,

    /// Enable notification sounds (default: false)
    #[serde(default)]
    pub sound: bool,
}

// ── Default value functions ──────────────────────────────────────────

fn default_work_duration() -> u64 {
    25
}
fn default_short_break() -> u64 {
    5
}
fn default_long_break() -> u64 {
    15
}
fn default_long_break_interval() -> u32 {
    4
}
fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work_duration: default_work_duration(),
            short_break: default_short_break(),
            long_break: default_long_break(),
            long_break_interval: default_long_break_interval(),
            auto_start_breaks: true,
            auto_start_pomodoros: false,
            notifications: true,
            sound: false,
        }
    }
}

impl Config {
    /// Returns the path to the config file: `~/.config/pomodomate/config.toml`
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("pomodomate");
        Ok(config_dir.join("config.toml"))
    }

    /// Load config from disk, or create default if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config at {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config at {}", path.display()))?;
            config.validate()?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Load config from a specific path.
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config at {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    /// Save the current config to disk, creating directories if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }

    /// Validate config values are sane.
    pub(crate) fn validate(&self) -> Result<()> {
        anyhow::ensure!(self.work_duration > 0, "work_duration must be > 0");
        anyhow::ensure!(self.short_break > 0, "short_break must be > 0");
        anyhow::ensure!(self.long_break > 0, "long_break must be > 0");
        anyhow::ensure!(
            self.long_break_interval > 0,
            "long_break_interval must be > 0"
        );
        Ok(())
    }
}
