<h1 align="center">
  🍅 Pomodomate CLI
</h1>

<p align="center">
  <strong>A beautiful Pomodoro timer for the terminal — featuring Domate, your animated tomato companion.</strong>
</p>

<p align="center">
  <a href="https://pomodomate.com">Website</a> •
  <a href="#installation">Install</a> •
  <a href="#usage">Usage</a> •
  <a href="#configuration">Config</a> •
  <a href="PRIVACY.md">Privacy</a>
</p>

<p align="center">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-blue.svg">
  <img alt="Rust" src="https://img.shields.io/badge/rust-stable-orange.svg">
  <img alt="Platform" src="https://img.shields.io/badge/platform-linux-lightgrey.svg">
  <img alt="Offline" src="https://img.shields.io/badge/offline-first-green.svg">
</p>

---

## ✨ Features

- 🍅 **Full Pomodoro Cycle** — Work → Short Break → Long Break, all automated
- 🎨 **Domate Mascot** — Animated Unicode tomato with ANSI 24-bit colors, right in your terminal
- 📊 **GitHub-style Heatmap** — Visualize your productivity over time
- 🔔 **Native Notifications** — Wayland-compatible (Dunst, Mako, SwayNC)
- ⚡ **Blazing Fast** — Single Rust binary, no runtime, instant startup
- 🔒 **100% Offline** — No telemetry, no accounts, no internet required
- ⚙️ **Configurable** — TOML config for durations, breaks, and behavior

## 📦 Installation

### Prebuilt Binaries (Linux)

Download the latest binary for x86_64 or aarch64 from the
[Releases page](https://github.com/Zapayox30/Pomodomate-cli/releases), then:

```bash
tar xzf pomodomate-*.tar.gz
sudo install -m 755 pomodomate /usr/local/bin/
```

### Cargo (Universal)

```bash
cargo install pomodomate-cli
```

### AUR (Arch Linux) — coming soon

```bash
yay -S pomodomate-cli
```

### From Source

```bash
git clone https://github.com/Zapayox30/Pomodomate-cli.git
cd Pomodomate-cli
cargo build --release
./target/release/pomodomate
```

> The package is named `pomodomate-cli`, but the installed command is simply `pomodomate`.

## 🚀 Usage

```bash
# Start a Pomodoro session
pomodomate

# Custom durations for this run only (config.toml is untouched)
pomodomate -w 50 -b 10        # 50-min focus, 10-min short breaks
pomodomate -l 20 -i 3         # 20-min long break after 3 pomodoros

# Run silently — no sound, no notifications
pomodomate --mute

# Print your stats without opening the timer (great for scripts/Waybar)
pomodomate stats
pomodomate stats --json

# Use custom config
pomodomate --config /path/to/config.toml
```

The interface adapts to your terminal: full mascot view in large windows,
a compact view in small splits, and a one-line mini mode in tiny panes.

## ⌨️ Keybindings

| Key | Action |
|-----|--------|
| `Space` | Start / Pause / Resume timer |
| `r` | Reset current timer |
| `s` | Skip to next phase |
| `h` | Toggle heatmap view |
| `+` / `-` | Add / remove one minute |
| `?` | Show help overlay |
| `q` | Quit (asks for confirmation while running) |

## ⚙️ Configuration

Config file: `~/.config/pomodomate/config.toml`

```toml
# Timer durations (in minutes)
work_duration = 25
short_break = 5
long_break = 15

# Number of pomodoros before a long break
long_break_interval = 4

# Behavior
auto_start_breaks = true
auto_start_pomodoros = false

# Notifications
notifications = true
sound = false
```

## 🗺️ Roadmap

| Phase | Status | Description |
|-------|--------|-------------|
| **Phase 1** — MVP Offline | 🚧 In Progress | Timer, Domate mascot, heatmap, notifications |
| **Phase 2** — Sync | 📋 Planned | Sync with pomodomate.com ecosystem |
| **Phase 3** — Domate Mode | 📋 Planned | Local camera-based distraction detection |

## 🌍 Ecosystem

Pomodomate CLI is part of the [Pomodomate.com](https://pomodomate.com) ecosystem:

- **pomodomate.com** — Web app (deployed)
- **Mobile app** — iOS & Android (in development)
- **pomodomate-cli** — This project ← you are here

## 📄 License

MIT License — Copyright 2025 [Pomodomate.com](https://pomodomate.com)

The source code is open source. The name "Pomodomate" and mascot "Domate" are trademarks of Pomodomate.com. See [LICENSE](LICENSE) for details.

## 🔒 Privacy

Zero telemetry. Zero tracking. Everything runs locally. See [PRIVACY.md](PRIVACY.md) for our full privacy guarantees.

---

<p align="center">
  Made with 🍅 by <a href="https://pomodomate.com">Pomodomate.com</a>
</p>
