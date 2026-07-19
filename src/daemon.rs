use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::config::Config;
use crate::engine::{Engine, Status};

/// Commands the daemon understands, one per line over the socket.
///
/// The protocol is deliberately plain text so it can be driven from a shell
/// with `socat` or `nc` while debugging.
pub const COMMANDS: &[&str] = &[
    "status", "toggle", "start", "pause", "resume", "reset", "skip", "quit",
];

/// Longest socket path the kernel will accept.
///
/// `sockaddr_un.sun_path` is 108 bytes on Linux and 104 on the BSDs, including
/// the trailing NUL. We check against the smaller figure so the error comes
/// from us, with an explanation, rather than from `bind` as `SUN_LEN`.
const MAX_SOCKET_PATH: usize = 103;

/// Longest command we will read from a client.
///
/// The longest valid command is six bytes; without a cap, a client that never
/// sends a newline grows the buffer until the daemon is OOM-killed.
const MAX_COMMAND_BYTES: u64 = 1024;

/// How many connections may be in flight at once.
const MAX_CONCURRENT_CLIENTS: usize = 32;

/// Path of the control socket.
///
/// Lives in the user's runtime directory so it is per-user, cleaned up by the
/// system on logout, and never world-writable. Falls back to a uid-suffixed
/// path under the temp dir when `XDG_RUNTIME_DIR` is unset, and can be
/// overridden with `POMODOMATE_SOCKET`.
pub fn socket_path() -> PathBuf {
    if let Some(path) = std::env::var_os("POMODOMATE_SOCKET") {
        return PathBuf::from(path);
    }
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("pomodomate.sock");
    }
    // No runtime dir: fall back to a private directory of our own under the
    // temp dir. The socket must not sit directly in a world-writable place,
    // where another user could pre-create the path and either block us or
    // impersonate the daemon.
    let uid = unsafe { libc_getuid() };
    std::env::temp_dir()
        .join(format!("pomodomate-{uid}"))
        .join("pomodomate.sock")
}

/// Make sure the socket's parent directory exists and only we can enter it.
///
/// Refuses to continue if the directory exists but belongs to someone else or
/// is open to other users — that is the shape of a squatting attempt.
fn prepare_socket_dir(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    if !parent.exists() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
        return Ok(());
    }

    let meta =
        fs::metadata(parent).with_context(|| format!("Failed to inspect {}", parent.display()))?;
    anyhow::ensure!(
        meta.is_dir(),
        "{} exists but is not a directory",
        parent.display()
    );

    // Only enforce ownership on directories we created; a user-chosen
    // XDG_RUNTIME_DIR or POMODOMATE_SOCKET location is their business.
    let ours = parent
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("pomodomate-"));
    if ours {
        let uid = unsafe { libc_getuid() };
        anyhow::ensure!(
            meta.uid() == uid,
            "{} is owned by uid {} rather than {uid} — refusing to use it",
            parent.display(),
            meta.uid()
        );
        if meta.mode() & 0o077 != 0 {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
                .with_context(|| format!("Failed to restrict {}", parent.display()))?;
        }
    }

    Ok(())
}

/// Remove a socket left behind by a dead daemon.
///
/// Only unlinks actual sockets: pointing `POMODOMATE_SOCKET` at a regular file
/// by mistake must never destroy it.
fn remove_stale_socket(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)
        .with_context(|| format!("Failed to inspect {}", path.display()))?;
    anyhow::ensure!(
        meta.file_type().is_socket(),
        "{} exists and is not a socket — refusing to delete it",
        path.display()
    );
    fs::remove_file(path)
        .with_context(|| format!("Failed to clear stale socket {}", path.display()))
}

/// Reject an over-long socket path before the kernel does, with advice the
/// user can act on.
fn check_socket_path(path: &Path) -> Result<()> {
    let len = path.as_os_str().len();
    if len > MAX_SOCKET_PATH {
        bail!(
            "socket path is too long ({len} bytes, limit {MAX_SOCKET_PATH}): {}\n\
             Set POMODOMATE_SOCKET to a shorter path, e.g. POMODOMATE_SOCKET=/tmp/pomodomate.sock",
            path.display()
        );
    }
    Ok(())
}

/// `getuid(2)` without pulling in a libc dependency for one call.
unsafe fn libc_getuid() -> u32 {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    unsafe { getuid() }
}

/// Whether a daemon is already listening on the socket.
fn daemon_is_live(path: &Path) -> bool {
    UnixStream::connect(path).is_ok()
}

/// Run the timer headlessly, serving commands over the socket.
///
/// Blocks until a client sends `quit` or the process is interrupted.
pub fn serve(config: Config) -> Result<()> {
    let path = socket_path();
    check_socket_path(&path)?;
    prepare_socket_dir(&path)?;

    if path.exists() {
        if daemon_is_live(&path) {
            bail!(
                "a Pomodomate daemon is already running on {}",
                path.display()
            );
        }
        // Left behind by a crash or a kill -9: nothing is listening, so the
        // socket file is safe to replace.
        remove_stale_socket(&path)?;
    }

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("Failed to bind socket {}", path.display()))?;

    // Do not inherit the process umask: the socket grants full control of the
    // timer and can fire the user's hooks, so it is owner-only regardless.
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("Failed to restrict {}", path.display()))?;

    let idle_timeout = Duration::from_secs(config.idle_timeout * 60);
    let engine = Arc::new(Mutex::new(Engine::new(config)?));

    // Pause the timer when the user walks away, where the compositor supports
    // it. Absent elsewhere, and the daemon runs exactly as before.
    if let Some(events) = crate::idle::watch(idle_timeout) {
        let watched = Arc::clone(&engine);
        std::thread::spawn(move || {
            while let Ok(event) = events.recv() {
                if let Ok(mut engine) = watched.lock() {
                    engine.handle_idle(event);
                }
            }
        });
    }

    // The clock runs on its own thread so a slow client cannot delay a tick.
    let ticker = Arc::clone(&engine);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        if let Ok(mut engine) = ticker.lock() {
            let _ = engine.tick();
        }
    });

    println!("🍅 Pomodomate daemon listening on {}", path.display());

    let shutdown = Arc::new(AtomicBool::new(false));
    let clients = Arc::new(AtomicUsize::new(0));

    // Terminating politely must not cost the user their in-flight pomodoro,
    // and must give teardown hooks (do-not-disturb, music) their turn.
    install_signal_handler(Arc::clone(&shutdown), path.clone())?;

    for stream in listener.incoming() {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        let stream = match stream {
            Ok(stream) => stream,
            // One bad connection should never take the daemon down.
            Err(_) => continue,
        };

        // Each connection gets its own thread, so a slow or stalled client
        // cannot keep everyone else waiting behind it.
        if clients.load(Ordering::SeqCst) >= MAX_CONCURRENT_CLIENTS {
            // Refuse rather than queue: unbounded threads is how a local
            // denial of service starts.
            continue;
        }
        clients.fetch_add(1, Ordering::SeqCst);

        let engine = Arc::clone(&engine);
        let shutdown = Arc::clone(&shutdown);
        let clients_guard = Arc::clone(&clients);
        let wake_path = path.clone();
        std::thread::spawn(move || {
            if let Ok(true) = handle_client(stream, &engine) {
                shutdown.store(true, Ordering::SeqCst);
                // Unblock the accept loop so it notices the flag.
                let _ = UnixStream::connect(&wake_path);
            }
            clients_guard.fetch_sub(1, Ordering::SeqCst);
        });
    }

    shutdown_engine(&engine);
    let _ = fs::remove_file(&path);
    println!("🍅 Pomodomate daemon stopped.");
    Ok(())
}

/// Record the in-flight phase and run teardown hooks before exiting.
fn shutdown_engine(engine: &Arc<Mutex<Engine>>) {
    let mut engine = lock(engine);
    engine.shut_down();
}

/// Take the engine lock, recovering from a poisoned mutex.
///
/// A panic elsewhere must not turn the daemon into a process that answers
/// nothing forever; the engine is plain data, so the state is still usable.
fn lock(engine: &Arc<Mutex<Engine>>) -> std::sync::MutexGuard<'_, Engine> {
    engine
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Exit cleanly on SIGINT/SIGTERM instead of dropping the session on the floor.
fn install_signal_handler(shutdown: Arc<AtomicBool>, path: PathBuf) -> Result<()> {
    let mut signals = signal_hook::iterator::Signals::new([
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
    ])
    .context("Failed to install signal handler")?;

    std::thread::spawn(move || {
        if signals.forever().next().is_some() {
            shutdown.store(true, Ordering::SeqCst);
            // Wake the accept loop so the normal shutdown path runs.
            let _ = UnixStream::connect(&path);
        }
    });

    Ok(())
}

/// Serve one connection. Returns `true` when the client asked the daemon to
/// shut down.
fn handle_client(stream: UnixStream, engine: &Arc<Mutex<Engine>>) -> Result<bool> {
    // A client that connects and says nothing must not wedge the daemon.
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream).take(MAX_COMMAND_BYTES);

    let mut line = String::new();
    reader.read_line(&mut line)?;
    let command = line.trim();

    let response = {
        let mut engine = lock(engine);
        apply(&mut engine, command)?
    };

    writeln!(writer, "{}", response.body)?;
    writer.flush()?;
    Ok(response.shutdown)
}

/// What to send back to a client, and whether to stop afterwards.
struct Response {
    body: String,
    shutdown: bool,
}

/// Apply a command to the engine and produce its reply.
fn apply(engine: &mut Engine, command: &str) -> Result<Response> {
    let body = match command {
        "status" => serde_json::to_string(&engine.status())?,
        "toggle" => {
            engine.toggle();
            "ok".to_string()
        }
        "start" | "resume" => {
            engine.resume();
            "ok".to_string()
        }
        "pause" => {
            engine.pause();
            "ok".to_string()
        }
        "reset" => {
            engine.reset();
            "ok".to_string()
        }
        "skip" => {
            engine.skip()?;
            "ok".to_string()
        }
        "quit" => {
            // The in-flight phase still happened, and teardown hooks must run.
            engine.shut_down();
            return Ok(Response {
                body: "ok".to_string(),
                shutdown: true,
            });
        }
        other => format!("error: unknown command {other:?}"),
    };

    Ok(Response {
        body,
        shutdown: false,
    })
}

/// Send one command to a running daemon and return its reply.
pub fn request(command: &str) -> Result<String> {
    let path = socket_path();
    check_socket_path(&path)?;
    let stream = UnixStream::connect(&path).with_context(|| {
        format!(
            "no Pomodomate daemon on {} — start one with `pomodomate daemon`",
            path.display()
        )
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;

    let mut writer = stream.try_clone()?;
    writeln!(writer, "{command}")?;
    writer.flush()?;

    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply.trim().to_string())
}

/// Ask the daemon for its current state.
///
/// The reply is sanitized before it reaches the caller: a status line usually
/// ends up echoed to a terminal or a status bar, and a rogue process bound to
/// the socket could otherwise smuggle escape sequences through the text
/// fields.
pub fn query_status() -> Result<Status> {
    let raw = request("status")?;
    let mut status: Status = serde_json::from_str(&raw)
        .with_context(|| format!("daemon returned an unexpected status reply: {raw}"))?;

    status.phase = sanitize(&status.phase);
    status.status = sanitize(&status.status);
    status.time = sanitize(&status.time);
    status.error = status.error.as_deref().map(sanitize);
    Ok(status)
}

/// Drop control characters, which is what an escape-sequence injection needs.
fn sanitize(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use uuid::Uuid;

    fn engine() -> Engine {
        let dir = std::env::temp_dir()
            .join("pomodomate-daemon-test")
            .join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        // Keep tests quiet and free of desktop side effects.
        let config = Config {
            notifications: false,
            sound: false,
            ..Default::default()
        };
        Engine::with_storage(config, Storage::at_dir(dir))
    }

    #[test]
    fn status_command_returns_parseable_json() {
        let mut e = engine();
        let reply = apply(&mut e, "status").unwrap();
        assert!(!reply.shutdown);

        let status: Status = serde_json::from_str(&reply.body).unwrap();
        assert_eq!(status.phase, "work");
        assert_eq!(status.status, "idle");
    }

    #[test]
    fn toggle_starts_the_timer() {
        let mut e = engine();
        apply(&mut e, "toggle").unwrap();
        assert_eq!(e.status().status, "running");
    }

    #[test]
    fn pause_and_resume_are_idempotent() {
        let mut e = engine();
        // Pausing an idle timer is a no-op rather than an error.
        apply(&mut e, "pause").unwrap();
        assert_eq!(e.status().status, "idle");

        apply(&mut e, "start").unwrap();
        apply(&mut e, "pause").unwrap();
        apply(&mut e, "pause").unwrap();
        assert_eq!(e.status().status, "paused");

        apply(&mut e, "resume").unwrap();
        apply(&mut e, "resume").unwrap();
        assert_eq!(e.status().status, "running");
    }

    #[test]
    fn skip_moves_to_the_next_phase() {
        let mut e = engine();
        apply(&mut e, "start").unwrap();
        apply(&mut e, "skip").unwrap();
        assert_eq!(e.status().phase, "short_break");
    }

    #[test]
    fn reset_returns_to_a_full_idle_phase() {
        let mut e = engine();
        apply(&mut e, "start").unwrap();
        apply(&mut e, "reset").unwrap();
        let status = e.status();
        assert_eq!(status.status, "idle");
        assert_eq!(status.time, "25:00");
    }

    #[test]
    fn quit_signals_shutdown() {
        let mut e = engine();
        assert!(apply(&mut e, "quit").unwrap().shutdown);
    }

    #[test]
    fn unknown_commands_report_an_error_without_dying() {
        let mut e = engine();
        let reply = apply(&mut e, "explode").unwrap();
        assert!(reply.body.starts_with("error:"), "got {}", reply.body);
        assert!(!reply.shutdown, "a bad command must not stop the daemon");
    }

    #[test]
    fn over_long_socket_paths_are_rejected_with_advice() {
        let long = PathBuf::from(format!("/tmp/{}/pomodomate.sock", "x".repeat(120)));
        let err = check_socket_path(&long).unwrap_err().to_string();
        assert!(err.contains("too long"), "got: {err}");
        assert!(
            err.contains("POMODOMATE_SOCKET"),
            "the error should tell the user how to fix it, got: {err}"
        );
    }

    #[test]
    fn socket_paths_within_the_limit_are_accepted() {
        assert!(check_socket_path(&PathBuf::from("/run/user/1000/pomodomate.sock")).is_ok());
    }

    #[test]
    fn sanitize_strips_terminal_escape_sequences() {
        // A rogue process bound to the socket must not be able to smuggle
        // escapes into a terminal or a status bar through the status text.
        let hostile = "\u{1b}]0;PWNED\u{7}work\u{1b}[31m";
        let clean = sanitize(hostile);
        assert!(!clean.contains('\u{1b}'), "escape survived: {clean:?}");
        assert!(!clean.chars().any(char::is_control));
        assert!(clean.contains("work"), "readable text should survive");
    }

    #[test]
    fn sanitize_leaves_ordinary_text_alone() {
        assert_eq!(sanitize("short_break"), "short_break");
        assert_eq!(sanitize("04:59"), "04:59");
    }

    #[test]
    fn a_regular_file_is_never_deleted_as_a_stale_socket() {
        let path = std::env::temp_dir().join(format!("pomodomate-precious-{}", Uuid::new_v4()));
        std::fs::write(&path, b"important").unwrap();

        let err = remove_stale_socket(&path).unwrap_err().to_string();
        assert!(err.contains("not a socket"), "got: {err}");
        assert!(path.exists(), "the file must survive");
        assert_eq!(std::fs::read(&path).unwrap(), b"important");

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn the_temp_fallback_uses_a_private_directory() {
        // SAFETY: single-threaded test; both variables are restored below.
        let runtime = std::env::var_os("XDG_RUNTIME_DIR");
        let socket = std::env::var_os("POMODOMATE_SOCKET");
        unsafe {
            std::env::remove_var("XDG_RUNTIME_DIR");
            std::env::remove_var("POMODOMATE_SOCKET");
        }

        let path = socket_path();
        let parent = path.parent().unwrap();
        assert!(
            parent
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("pomodomate-"),
            "the socket must live in a directory of ours, not directly in /tmp: {}",
            path.display()
        );

        unsafe {
            if let Some(value) = runtime {
                std::env::set_var("XDG_RUNTIME_DIR", value);
            }
            if let Some(value) = socket {
                std::env::set_var("POMODOMATE_SOCKET", value);
            }
        }
    }

    #[test]
    fn socket_path_follows_the_runtime_dir() {
        // SAFETY: single-threaded test, and the variables are restored below.
        let previous = std::env::var_os("XDG_RUNTIME_DIR");
        let previous_override = std::env::var_os("POMODOMATE_SOCKET");
        unsafe { std::env::remove_var("POMODOMATE_SOCKET") };
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", "/run/user/test") };
        assert_eq!(
            socket_path(),
            PathBuf::from("/run/user/test/pomodomate.sock")
        );
        match previous {
            Some(value) => unsafe { std::env::set_var("XDG_RUNTIME_DIR", value) },
            None => unsafe { std::env::remove_var("XDG_RUNTIME_DIR") },
        }
        if let Some(value) = previous_override {
            unsafe { std::env::set_var("POMODOMATE_SOCKET", value) };
        }
    }
}
