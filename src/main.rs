mod app;
mod config;
mod storage;
mod timer;
mod ui;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use app::App;
use config::Config;

/// 🍅 Pomodomate CLI — A beautiful Pomodoro timer for the terminal
#[derive(Parser, Debug)]
#[command(
    name = "pomodomate",
    version,
    about = "🍅 A beautiful Pomodoro timer for the terminal — featuring Domate, your animated tomato companion",
    long_about = None
)]
struct Cli {
    /// Path to a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Enable Domate mode (local distraction detection) [Phase 3 — not yet available]
    #[arg(long, hide = true)]
    domate: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let config = if let Some(config_path) = cli.config {
        Config::load_from(&config_path)
            .with_context(|| format!("Failed to load config from {}", config_path.display()))?
    } else {
        Config::load().context("Failed to load config")?
    };

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
