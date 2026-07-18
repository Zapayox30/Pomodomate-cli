mod app;
mod config;
mod daemon;
mod engine;
mod hooks;
mod idle;
mod sound;
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
    after_help = "EXAMPLES:\n  pomodomate                 start with your saved config\n  pomodomate -w 50 -b 10     50-minute focus, 10-minute breaks (this run only)\n  pomodomate --mute          run silently, no sound or notifications\n  pomodomate -t tesis        tag this run's sessions as \"tesis\"\n  pomodomate stats           print your stats without opening the timer\n  pomodomate stats --by-tag  see which tags your pomodoros went to",
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

    /// Tag the sessions from this run (repeatable, or comma separated)
    #[arg(short = 't', long = "tag", value_name = "TAG")]
    tags: Vec<String>,

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

        /// Only count sessions carrying this tag
        #[arg(short = 't', long = "tag", value_name = "TAG")]
        tag: Option<String>,

        /// Break the totals down by tag instead of showing a summary
        #[arg(long)]
        by_tag: bool,
    },

    /// Run the timer in the background, controllable from anywhere
    Daemon,

    /// Send a command to the running daemon (toggle, pause, resume, skip, reset, quit)
    Ctl {
        /// One of: toggle, start, pause, resume, reset, skip, quit
        command: String,
    },

    /// Print the running daemon's state, for status bars
    Status {
        /// Template to render, e.g. "{icon} {time}" or "{phase} {percent}%"
        #[arg(long, value_name = "TEMPLATE")]
        format: Option<String>,

        /// Output the raw status object instead of a formatted line
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Subcommands that talk to an existing daemon or to the history file need
    // no config of their own.
    match &cli.command {
        Some(Command::Stats { json, tag, by_tag }) => {
            return print_stats(*json, tag.as_deref(), *by_tag);
        }
        Some(Command::Ctl { command }) => return run_ctl(command),
        Some(Command::Status { format, json }) => {
            return print_daemon_status(format.as_deref(), *json);
        }
        _ => {}
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
    if !cli.tags.is_empty() {
        config.tags = storage::parse_tags(&cli.tags);
    } else {
        // Normalize whatever came from config.toml so stored tags always look
        // the same regardless of where they were written.
        config.tags = storage::parse_tags(&config.tags);
    }
    config
        .validate()
        .context("Invalid command-line options (durations must be greater than 0)")?;

    // The daemon runs the same engine as the TUI, just without a screen.
    if matches!(cli.command, Some(Command::Daemon)) {
        return daemon::serve(config);
    }

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
        app.engine.timer.pomodoros_completed
    );

    Ok(())
}

/// `pomodomate ctl <command>` — forward a command to the daemon.
fn run_ctl(command: &str) -> Result<()> {
    if !daemon::COMMANDS.contains(&command) {
        anyhow::bail!(
            "unknown command {command:?} — try one of: {}",
            daemon::COMMANDS.join(", ")
        );
    }

    let reply = daemon::request(command)?;
    // `status` answers with JSON; the rest just acknowledge.
    if !reply.is_empty() && reply != "ok" {
        println!("{reply}");
    }
    Ok(())
}

/// `pomodomate status` — render the daemon's state for a status bar.
fn print_daemon_status(format: Option<&str>, json: bool) -> Result<()> {
    let status = daemon::query_status()?;

    if json {
        println!("{}", serde_json::to_string(&status)?);
    } else {
        println!("{}", status.render(format.unwrap_or("{icon} {time}")));
    }

    Ok(())
}

/// `pomodomate stats` — print aggregate stats and exit.
fn print_stats(json: bool, tag: Option<&str>, by_tag: bool) -> Result<()> {
    let storage = Storage::new()?;

    if by_tag {
        return print_tag_breakdown(&storage, json);
    }

    let stats = storage.stats_tagged(tag)?;

    if json {
        println!("{}", serde_json::to_string(&stats)?);
    } else {
        match tag {
            Some(tag) => println!("🍅 Pomodomate — your focus stats for #{tag}"),
            None => println!("🍅 Pomodomate — your focus stats"),
        }
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

/// `pomodomate stats --by-tag` — completed pomodoros per tag.
fn print_tag_breakdown(storage: &Storage, json: bool) -> Result<()> {
    let totals = storage.tag_totals()?;

    if json {
        let map: serde_json::Map<String, serde_json::Value> = totals
            .into_iter()
            .map(|(tag, count)| (tag, serde_json::Value::from(count)))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::Value::Object(map))?
        );
        return Ok(());
    }

    if totals.is_empty() {
        println!("🍅 No tagged pomodoros yet.");
        println!();
        println!("  Tag a run with:  pomodomate --tag tesis");
        return Ok(());
    }

    println!("🍅 Pomodomate — pomodoros by tag (last 365 days)");
    println!();
    let width = totals.iter().map(|(t, _)| t.len()).max().unwrap_or(0);
    for (tag, count) in totals {
        println!("  {tag:<width$}  {count:>4}");
    }

    Ok(())
}
