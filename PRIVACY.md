# 🔒 Privacy Policy — Pomodomate CLI

**Version:** 0.1.0  
**Last updated:** 2025

## Our Promise

Pomodomate CLI is designed with a **privacy-first, offline-first** philosophy. We believe your productivity data belongs to you and only you.

## Data Collection

### What we collect: **Nothing.**

Pomodomate CLI:

- ❌ **No telemetry** — Zero analytics, zero tracking, zero phone-home
- ❌ **No network requests** — The binary never connects to the internet (unless you explicitly enable sync in Phase 2)
- ❌ **No accounts required** — No sign-up, no login, no email
- ❌ **No third-party services** — No Firebase, no Sentry, no Mixpanel, nothing

### What stays on your machine

All data is stored **locally** in standard XDG directories:

| Data | Location | Format |
|------|----------|--------|
| Configuration | `~/.config/pomodomate/config.toml` | TOML |
| Session history | `~/.local/share/pomodomate/sessions.jsonl` | JSON Lines |

You own these files. You can read, edit, backup, or delete them at any time.

## Phase 2: Optional Sync (Future)

When sync is available (Phase 2), it will be:

- **Opt-in only** — You must explicitly enable it
- **Transparent** — The CLI will clearly show when data is being sent
- **Documented** — The exact API endpoints and data format will be public
- **Revocable** — You can disable sync and delete remote data at any time

The sync endpoint will be `api.pomodomate.com` and nothing else.

## Phase 3: Domate Mode (Future)

The camera-based distraction detection feature (`--domate`) will:

- ✅ Process all video frames **100% locally** on your machine
- ❌ **Never** save, store, or transmit any camera frames
- ❌ **Never** send any visual data to any server
- ✅ Use only local ML inference — no cloud AI services

## Verification

This is open source software under the MIT license. Every claim above can be verified by auditing the source code in the `src/` directory. We encourage and welcome code audits.

```bash
# Verify no network code exists in the MVP
grep -r "reqwest\|hyper\|curl\|http\|fetch" src/
# Should return zero results in Phase 1
```

## Contact

If you have privacy concerns, please open an issue on GitHub or contact us at [pomodomate.com](https://pomodomate.com).

---

*Pomodomate CLI — Your data, your machine, your rules.*
