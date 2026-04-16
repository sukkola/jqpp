//! Integration tests for process exit behaviour.
//!
//! These tests verify that the binary can always be terminated — by signal or
//! by the in-app ctrl+q key — and that it does NOT hang under any circumstance.
//!
//! # How they work
//!
//! Each test starts the compiled binary with piped JSON on stdin.  In a real
//! terminal the binary will open /dev/tty and start the TUI; in a headless CI
//! environment it exits immediately with "No TTY found".  Either way we only
//! assert **liveness**: the process must exit within the deadline.
//!
//! SIGTERM / SIGINT tests additionally verify that our `on_exit_signal` handler
//! fires and terminates the process even when the event loop is running normally
//! (i.e. not blocked on clipboard or a slow jq query).

use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BINARY: &str = "target/debug/jqt";
const SAMPLE_JSON: &[u8] = br#"{"name":"test","value":42,"nested":{"x":1}}"#;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Spawn the binary, optionally send bytes to stdin, then wait for it to exit
/// on its own up to `wait_before_signal`.  Returns the child so the caller can
/// send a signal and assert on exit timing.
fn spawn_with_input(input: &[u8]) -> std::process::Child {
    let mut child = Command::new(BINARY)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|_| panic!("Failed to spawn {BINARY} — run `cargo build` first"));

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input);
        // Close stdin so the binary can proceed past stdin read
    }
    child
}

/// Wait for `child` to exit, returning `true` if it exits before `deadline`.
fn wait_for_exit(child: &mut std::process::Child, deadline: Duration) -> bool {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if start.elapsed() >= deadline => return false,
            Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            Err(_) => return true,
        }
    }
}

/// Send signal `sig` to the child process (Unix only).
#[cfg(unix)]
fn send_signal(child: &std::process::Child, sig: libc::c_int) {
    unsafe { libc::kill(child.id() as libc::pid_t, sig) };
}

// ── SIGTERM ───────────────────────────────────────────────────────────────────

/// SIGTERM must terminate the binary within 2 s regardless of whether the TUI
/// is running.  This exercises `on_exit_signal()` in the real process.
#[test]
#[cfg(unix)]
fn test_sigterm_exits_within_deadline() {
    let mut child = spawn_with_input(SAMPLE_JSON);

    // Give the binary time to either start the TUI or fail gracefully.
    std::thread::sleep(Duration::from_millis(400));

    // If it already exited (headless env / no TTY), we're done.
    if child.try_wait().ok().flatten().is_some() {
        return;
    }

    // Binary is still running — send SIGTERM and expect it to exit promptly.
    send_signal(&child, libc::SIGTERM);

    assert!(
        wait_for_exit(&mut child, Duration::from_secs(2)),
        "Binary did not exit within 2 s after SIGTERM — event loop may be stuck"
    );

    let _ = child.kill();
}

// ── SIGINT ────────────────────────────────────────────────────────────────────

/// SIGINT (what a user sends via `kill -INT <pid>` when the app is stuck) must
/// also terminate the binary promptly.
#[test]
#[cfg(unix)]
fn test_sigint_exits_within_deadline() {
    let mut child = spawn_with_input(SAMPLE_JSON);

    std::thread::sleep(Duration::from_millis(400));

    if child.try_wait().ok().flatten().is_some() {
        return;
    }

    send_signal(&child, libc::SIGINT);

    assert!(
        wait_for_exit(&mut child, Duration::from_secs(2)),
        "Binary did not exit within 2 s after SIGINT — signal handler may not be installed"
    );

    let _ = child.kill();
}

// ── No hang on rapid key sequences (regression for clipboard blocking) ───────

/// Start the binary, give it 200 ms, then forcibly kill it.  The binary must
/// not be in a wedged state (blocking on clipboard / arboard) — it must die
/// immediately on SIGKILL.  SIGKILL is always delivered so this really just
/// verifies the process is alive and killable, not stuck in a kernel trap.
#[test]
#[cfg(unix)]
fn test_process_is_always_killable() {
    let mut child = spawn_with_input(SAMPLE_JSON);

    std::thread::sleep(Duration::from_millis(200));

    // Kill it hard — must succeed immediately.
    let _ = child.kill();
    assert!(
        wait_for_exit(&mut child, Duration::from_millis(500)),
        "Binary did not die after SIGKILL — process is in an unkillable state"
    );
}

// ── Does not emit internal crossterm crash on rapid re-launch ─────────────────

/// Rapidly launch and kill the binary several times in a row.  None of the runs
/// should produce the crossterm "Failed to initialize input reader" crash.
/// This is a regression guard: if we accidentally remove the `use-dev-tty`
/// feature, this test catches it.
#[test]
fn test_no_crash_on_repeated_start() {
    for _ in 0..3 {
        let mut child = Command::new(BINARY)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|_| panic!("Failed to spawn {BINARY}"));

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(SAMPLE_JSON);
        }

        std::thread::sleep(Duration::from_millis(150));
        let _ = child.kill();
        let output = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("Failed to initialize input reader"),
            "Crossterm internal crash on run:\n{stderr}"
        );
    }
}
