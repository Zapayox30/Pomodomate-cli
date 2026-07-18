mod app;
mod config;
mod storage;
mod theme;
mod timer;
mod ui;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use app::App;
use config::Config;
use storage::Storage;

/// 🍅 Pomodomate CLI — A beautiful Pomodoro timer for the terminal
#[derive(Parser, Debug)]
#[command(
    name = "pomodomate",
    version,
    about = "🍅 A beautiful Pomodoro timer for the terminal — featuring Domate, your animated tomato companion",
    after_help = "EXAMPLES:\n  pomodomate                 start with your saved config\n  pomodomate -w 50 -b 10     50-minute focus, 10-minute breaks (this run only)\n  pomodomate --mute          run silently, no sound or notifications\n  pomodomate stats           print your stats without opening the timer",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Work duration in minutes, just for this run
    #[arg(short = 'w', long, value_name = "MIN")]
    work: Option<u64>,

    /// Short break duration in minutes, just for this run
    #[arg(short = 'b', long, value_name = "MIN")]
    short_break: Option<u64>,

    /// Long break duration in minutes, just for this run
    #[arg(short = 'l', long, value_name = "MIN")]
    long_break: Option<u64>,

    /// Pomodoros before a long break, just for this run
    #[arg(short = 'i', long, value_name = "N")]
    interval: Option<u32>,

    /// Disable sound and desktop notifications for this run
    #[arg(long)]
    mute: bool,

    /// Hide the Domate mascot for this run
    #[arg(long)]
    no_mascot: bool,

    /// Theme name, just for this run ("default", "nord", "dracula", "gruvbox", "monochrome")
    #[arg(long, value_name = "THEME")]
    theme: Option<String>,

    /// Enable Domate mode (local distraction detection) [Phase 3 — not yet available]
    #[arg(long, hide = true)]
    domate: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print your productivity stats without opening the timer
    Stats {
        /// Output a single JSON object (for scripts and status bars)
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Command::Stats { json }) = cli.command {
        return print_stats(json);
    }

    // Load configuration
    let mut config = if let Some(config_path) = cli.config {
        Config::load_from(&config_path)
            .with_context(|| format!("Failed to load config from {}", config_path.display()))?
    } else {
        Config::load().context("Failed to load config")?
    };

    // Apply one-run command-line overrides (config.toml is never modified)
    if let Some(work) = cli.work {
        config.work_duration = work;
    }
    if let Some(short_break) = cli.short_break {
        config.short_break = short_break;
    }
    if let Some(long_break) = cli.long_break {
        config.long_break = long_break;
    }
    if let Some(interval) = cli.interval {
        config.long_break_interval = interval;
    }
    if cli.mute {
        config.sound = false;
        config.notifications = false;
    }
    if cli.no_mascot {
        config.show_mascot = false;
    }
    if let Some(theme) = cli.theme {
        config.theme = theme;
    }
    config
        .validate()
        .context("Invalid command-line options (durations must be greater than 0)")?;

    // Initialize the app
    let mut app = App::new(config).context("Failed to initialize Pomodomate")?;

    // Setup terminal
    let mut terminal = ratatui::init();

    // Run the app
    let result = app.run(&mut terminal);

    // Restore terminal (always, even on error)
    ratatui::restore();

    // Propagate any error from the app
    result.context("Pomodomate encountered an error")?;

    println!(
        "🍅 Thanks for using Pomodomate! You completed {} pomodoros. See you next time!",
        app.timer.pomodoros_completed
    );

    Ok(())
}

/// `pomodomate stats` — print aggregate stats and exit.
fn print_stats(json: bool) -> Result<()> {
    let stats = Storage::new()?.stats()?;

    if json {
        println!("{}", serde_json::to_string(&stats)?);
    } else {
        println!("🍅 Pomodomate — your focus stats");
        println!();
        println!("  Today:           {:>4} pomodoros", stats.today);
        println!("  Last 7 days:     {:>4} pomodoros", stats.week);
        println!("  Last 365 days:   {:>4} pomodoros", stats.year);
        println!("  Active days:     {:>4}", stats.active_days);
        println!("  Current streak:  {:>4} days 🔥", stats.current_streak);
        println!("  Best streak:     {:>4} days", stats.best_streak);
    }

    Ok(())
}
