# Waybar integration

Show your Pomodoro stats in Waybar using `pomodomate stats --json`.

## Simple: today's pomodoro count

Add to `~/.config/waybar/config.jsonc`:

```jsonc
"custom/pomodomate": {
    "exec": "pomodomate stats --json | jq -r .today",
    "format": "🍅 {}",
    "interval": 60,
    "tooltip": false
}
```

## Rich: count in the bar, details on hover

Requires `jq`:

```jsonc
"custom/pomodomate": {
    "exec": "pomodomate stats --json | jq --unbuffered --compact-output '{text: (.today | tostring), tooltip: (\"Today: \\(.today) 🍅  ·  Week: \\(.week)  ·  Streak: \\(.current_streak) days 🔥\")}'",
    "return-type": "json",
    "format": "🍅 {}",
    "interval": 60
}
```

Then add `"custom/pomodomate"` to `modules-right` (or wherever you prefer):

```jsonc
"modules-right": ["custom/pomodomate", "clock", "tray"]
```

## Available JSON fields

`pomodomate stats --json` outputs:

```json
{"today":3,"week":12,"year":87,"active_days":23,"best_streak":5,"current_streak":2}
```

| Field | Meaning |
|-------|---------|
| `today` | Completed pomodoros today |
| `week` | Completed pomodoros in the last 7 days |
| `year` | Completed pomodoros in the last 365 days |
| `active_days` | Days with at least one pomodoro (last 365 days) |
| `best_streak` | Longest streak of consecutive active days |
| `current_streak` | Consecutive active days ending today (yesterday counts until today ends) |
