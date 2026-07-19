use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
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
    let uid = unsafe { libc_getuid() };
    std::env::temp_dir().join(format!("pomodomate-{uid}.sock"))
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

    if path.exists() {
        if daemon_is_live(&path) {
            bail!(
                "a Pomodomate daemon is already running on {}",
                path.display()
            );
        }
        // Left behind by a crash or a kill -9: nothing is listening, so the
        // socket file is safe to replace.
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to clear stale socket {}", path.display()))?;
    }

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("Failed to bind socket {}", path.display()))?;

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

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            // One bad connection should never take the daemon down.
            Err(_) => continue,
        };

        match handle_client(stream, &engine) {
            Ok(true) => break, // client asked us to quit
            Ok(false) => {}
            Err(_) => continue,
        }
    }

    let _ = std::fs::remove_file(&path);
    println!("🍅 Pomodomate daemon stopped.");
    Ok(())
}

/// Serve one connection. Returns `true` when the client asked the daemon to
/// shut down.
fn handle_client(stream: UnixStream, engine: &Arc<Mutex<Engine>>) -> Result<bool> {
    // A client that connects and says nothing must not wedge the daemon.
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);

    let mut line = String::new();
    reader.read_line(&mut line)?;
    let command = line.trim();

    let response = {
        let mut engine = engine
            .lock()
            .map_err(|_| anyhow::anyhow!("engine lock poisoned"))?;
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
            // The in-flight phase still happened; do not drop it on exit.
            engine.abandon_phase();
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
pub fn query_status() -> Result<Status> {
    let raw = request("status")?;
    serde_json::from_str(&raw)
        .with_context(|| format!("daemon returned an unexpected status reply: {raw}"))
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
