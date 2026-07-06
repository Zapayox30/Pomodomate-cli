# Usability Improvements — Design

**Date:** 2026-07-06 · **Status:** Approved

Goal: make Pomodomate CLI friendly and easy to use on first contact, without new dependencies.

## 1. Quick CLI flags (`src/main.rs`)

- `-w/--work <MIN>`, `-b/--short-break <MIN>`, `-l/--long-break <MIN>`, `-i/--interval <N>` override the loaded config for this run only (config.toml is never written).
- `--mute` disables both sound and notifications for this run.
- Overrides are validated with the same rules as the config file (all values > 0).

## 2. In-app help & shortcuts (`app.rs`, `timer.rs`, `ui/`)

- `?` toggles a centered help overlay listing every keybinding. Any key closes it.
- `+` (or `=`) / `-` add or subtract one minute from the current phase.
  `Timer::adjust(minutes)` keeps `remaining >= 1 min` and keeps `total_duration >= remaining` so progress stays in [0, 1]. Unit-tested.
- Pressing `q`/`Esc` while the timer is **running** arms a confirmation ("press q again to quit") shown in the footer; a second `q` quits, any other key cancels. When idle/paused, `q` quits immediately.

## 3. `pomodomate stats` subcommand (`main.rs`, `storage.rs`)

- Prints today / last-7-days / last-365-days completed pomodoros, active days, best streak, and current streak. No TUI.
- `--json` prints the same as a single JSON object for scripts and Waybar widgets.
- Aggregation lives in `Storage::stats()` returning a serializable `Stats` struct; `calculate_streak` moves from `ui/heatmap.rs` to `storage.rs` (plus `current_streak`). Unit-tested.

## 4. Responsive layout (`ui/mod.rs`)

- Pure function `layout_mode(width, height) -> Full | Compact | Mini`, unit-tested:
  - **Full** (`h >= 32`): current layout with mascot.
  - **Compact** (`15 <= h < 32`): drops the mascot (today it gets clipped mid-body in small terminals).
  - **Mini** (`h < 15`): one-line timer + progress bar + footer.
- If `width < 34`, the big box-drawing digits are replaced by a plain `MM:SS` line in Full/Compact.

## Testing

TDD for all logic: `Timer::adjust`, `Storage::stats`/streaks, `layout_mode`. Drawing code and clap wiring verified by running the binary (`--help`, `stats`, TUI smoke test).

## Out of scope

Localization, config editing from the CLI, themes.
