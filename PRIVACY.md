# 🔒 Privacy Policy — Pomodomate CLI

**Version:** 0.3.0  
**Last updated:** July 2026

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

The session history records **when** you worked — timestamps, phase, duration
and any tags you chose. It never records what you were working on beyond those
tags.

### The control socket

Running `pomodomate daemon` creates a UNIX socket at
`$XDG_RUNTIME_DIR/pomodomate.sock` (or a private `0700` directory under your
temp dir if that variable is unset). It is created with mode `0600`, so only
your user can connect. It is local only — a UNIX socket cannot be reached over
a network — and it is removed when the daemon stops.

## Hooks: the one way Pomodomate can talk to the outside world

The claims above are about the binary itself. **You** can extend it: the
`[hooks]` section of your `config.toml` runs shell commands when the timer
changes phase, and those commands can do anything you can do — including
sending data over the network, because that is the point of the feature.

This matters in one specific situation: a `config.toml` you did not write.
Configuration files get shared in dotfiles repositories, pasted from blog
posts and bundled inside project repositories, and `pomodomate --config
./config.toml` will happily run whatever hooks that file defines, with your
permissions and no prompt.

Treat a `config.toml` the way you would treat a shell script someone sent you:
read the `[hooks]` section before running it. Pomodomate ships with no hooks
configured, so this only ever applies to commands you or someone else put
there deliberately. See [docs/hooks.md](docs/hooks.md).

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

# See exactly what the binary links against — no TLS, no HTTP stack
ldd $(which pomodomate)

# Review any hooks configured on your machine
grep -A5 '\[hooks\]' ~/.config/pomodomate/config.toml
```

## Contact

If you have privacy concerns, please open an issue on GitHub or contact us at [pomodomate.com](https://pomodomate.com).

---

*Pomodomate CLI — Your data, your machine, your rules.*
