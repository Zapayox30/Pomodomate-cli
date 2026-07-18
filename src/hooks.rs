use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

/// Shell commands run when the timer crosses a phase boundary.
///
/// Each hook is an arbitrary shell line executed with `sh -c`, so pipes and
/// `&&` work as usual. Hooks are fire-and-forget: Pomodomate never waits for
/// them and never reports their exit status, because a slow or broken hook
/// must not stall the timer.
///
/// A typical focus setup on Wayland:
///
/// ```toml
/// [hooks]
/// work_start = "swaync-client -d"
/// work_end = "swaync-client -d"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Hooks {
    /// Runs when a work phase starts (or resumes from idle).
    #[serde(default)]
    pub work_start: Option<String>,

    /// Runs when a work phase ends, whether completed or skipped.
    #[serde(default)]
    pub work_end: Option<String>,

    /// Runs when a short or long break starts.
    #[serde(default)]
    pub break_start: Option<String>,

    /// Runs when a short or long break ends, whether completed or skipped.
    #[serde(default)]
    pub break_end: Option<String>,
}

/// Context passed to a hook as environment variables.
///
/// Hooks receive this instead of positional arguments so that adding a field
/// later cannot break someone's existing shell line.
#[derive(Debug, Clone)]
pub struct HookContext {
    /// `work`, `short_break` or `long_break`.
    pub phase: String,
    /// Work pomodoros completed so far in this run.
    pub pomodoros: u32,
    /// Intended length of the phase, in minutes.
    pub duration_minutes: u64,
    /// Tags attached to the current run, comma separated.
    pub tags: String,
    /// `true` when the phase ran to completion rather than being skipped.
    pub completed: bool,
}

/// Build the `sh -c` invocation for a hook without running it.
///
/// Kept separate from [`spawn`] so the environment wiring can be tested
/// without executing arbitrary commands.
fn build_command(line: &str, ctx: &HookContext) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(line)
        .env("POMODOMATE_PHASE", &ctx.phase)
        .env("POMODOMATE_POMODOROS", ctx.pomodoros.to_string())
        .env("POMODOMATE_DURATION", ctx.duration_minutes.to_string())
        .env("POMODOMATE_TAGS", &ctx.tags)
        .env("POMODOMATE_COMPLETED", ctx.completed.to_string())
        // The TUI owns the terminal: a hook writing to stdout would corrupt
        // the frame, so all three streams go to /dev/null.
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

/// Run a hook without blocking the timer.
///
/// The child is reaped on a detached thread so a long-running hook does not
/// accumulate as a zombie process. Failures are deliberately silent: a typo in
/// someone's config should not take down their pomodoro.
pub fn spawn(line: &str, ctx: &HookContext) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }

    if let Ok(mut child) = build_command(line, ctx).spawn() {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> HookContext {
        HookContext {
            phase: "work".to_string(),
            pomodoros: 3,
            duration_minutes: 25,
            tags: "tesis,rust".to_string(),
            completed: true,
        }
    }

    #[test]
    fn hooks_default_to_none() {
        let hooks = Hooks::default();
        assert!(hooks.work_start.is_none());
        assert!(hooks.work_end.is_none());
        assert!(hooks.break_start.is_none());
        assert!(hooks.break_end.is_none());
    }

    #[test]
    fn command_passes_context_through_the_environment() {
        let cmd = build_command("true", &ctx());
        let envs: Vec<_> = cmd
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                )
            })
            .collect();

        assert!(envs.contains(&("POMODOMATE_PHASE".into(), "work".into())));
        assert!(envs.contains(&("POMODOMATE_POMODOROS".into(), "3".into())));
        assert!(envs.contains(&("POMODOMATE_DURATION".into(), "25".into())));
        assert!(envs.contains(&("POMODOMATE_TAGS".into(), "tesis,rust".into())));
        assert!(envs.contains(&("POMODOMATE_COMPLETED".into(), "true".into())));
    }

    #[test]
    fn command_runs_through_a_shell_so_pipes_work() {
        let cmd = build_command("echo a | tr a b", &ctx());
        assert_eq!(cmd.get_program(), "sh");
        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert_eq!(args, vec!["-c", "echo a | tr a b"]);
    }

    #[test]
    fn empty_hook_line_does_not_spawn() {
        // Nothing to assert beyond "this must not panic or hang".
        spawn("", &ctx());
        spawn("   ", &ctx());
    }

    #[test]
    fn hook_actually_executes_the_command() {
        let dir = std::env::temp_dir().join(format!("pomodomate-hook-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let marker = dir.join("ran");

        // The hook writes the phase it was given, proving both execution and
        // environment propagation.
        spawn(
            &format!("printf %s \"$POMODOMATE_PHASE\" > {}", marker.display()),
            &ctx(),
        );

        // Hooks are asynchronous, so poll briefly instead of sleeping blindly.
        let mut contents = String::new();
        for _ in 0..50 {
            if let Ok(read) = std::fs::read_to_string(&marker) {
                if !read.is_empty() {
                    contents = read;
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(contents, "work", "hook should have run with its context");
    }
}
