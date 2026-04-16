//! Integration tests for piped stdin support.
//!
//! These tests run the compiled binary with stdin piped.  In a headless test
//! environment `/dev/tty` is unavailable, so the binary will exit with
//! "No TTY found …" — a controlled error.  What we assert is that the OLD
//! crossterm crash ("Failed to initialize input reader") never appears, and
//! that JSON parse errors are reported cleanly.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Spawn jqt with piped stdin (and optionally extra args), wait up to
/// `timeout` for it to exit on its own, then kill it and collect output.
fn run_piped(
    input: &[u8],
    extra_args: &[&str],
    timeout: Duration,
) -> (Vec<u8>, Vec<u8>) {
    let mut child = Command::new("target/debug/jqt")
        .args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn target/debug/jqt — run `cargo build` first");

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input);
    }

    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break,
        }
    }

    let output = child.wait_with_output().expect("wait_with_output failed");
    (output.stdout, output.stderr)
}

// ──────────────────────────────────────────────────────────────────────────────

/// The primary regression test: the binary must NEVER emit the crossterm
/// "Failed to initialize input reader" internal crash when given piped stdin.
/// Previously this happened because mio/kqueue couldn't register the TTY fd.
///
/// In a headless test environment (no /dev/tty) the binary exits early with
/// "No TTY found …" — that's fine.  In a real terminal it starts normally.
/// Either way, the crossterm crash must not appear.
#[test]
fn pipe_no_crossterm_init_crash() {
    let (_, stderr) = run_piped(b"{}", &["--debug"], Duration::from_secs(3));
    let stderr_str = String::from_utf8_lossy(&stderr);
    assert!(
        !stderr_str.contains("Failed to initialize input reader"),
        "Crossterm internal crash detected — the mio/kqueue regression is back:\n{stderr_str}"
    );
}

/// Same assertion, no --debug flag (tests the non-debug error path).
#[test]
fn pipe_no_crossterm_init_crash_nodebug() {
    let (_, stderr) = run_piped(b"{}", &[], Duration::from_secs(3));
    let stderr_str = String::from_utf8_lossy(&stderr);
    assert!(
        !stderr_str.contains("Failed to initialize input reader"),
        "Crossterm crash (no --debug):\n{stderr_str}"
    );
}

/// Invalid JSON piped in must produce a parse error, not the crossterm crash.
/// Note: in a headless environment the binary exits with "No TTY found" before
/// parsing JSON, which also does NOT contain "Failed to initialize input reader".
#[test]
fn pipe_invalid_json_no_crossterm_crash() {
    let (_, stderr) = run_piped(b"not json {{{{", &[], Duration::from_secs(3));
    let stderr_str = String::from_utf8_lossy(&stderr);
    assert!(
        !stderr_str.contains("Failed to initialize input reader"),
        "Crossterm crash on invalid JSON input:\n{stderr_str}"
    );
}

/// JSON array input must not cause a crossterm crash.
#[test]
fn pipe_json_array_no_crossterm_crash() {
    let (_, stderr) = run_piped(b"[1,2,3]", &[], Duration::from_secs(3));
    let stderr_str = String::from_utf8_lossy(&stderr);
    assert!(
        !stderr_str.contains("Failed to initialize input reader"),
        "Crossterm crash on JSON array input:\n{stderr_str}"
    );
}

/// The binary must exit with a clear error (not a panic) when stdin is piped
/// but there is no controlling terminal available.
#[test]
fn pipe_no_tty_exits_cleanly() {
    let (_, stderr) = run_piped(b"{}", &[], Duration::from_secs(3));
    let stderr_str = String::from_utf8_lossy(&stderr);
    // The process should either:
    //   (a) exit cleanly with "No TTY found" (headless environment), or
    //   (b) start the TUI successfully (real terminal).
    // It must NOT panic.
    assert!(
        !stderr_str.contains("panicked"),
        "Binary panicked on piped input:\n{stderr_str}"
    );
}

/// When JSON is valid and a TTY is available (tested via JQT_SKIP_TTY_CHECK to
/// simulate headless), the JSON parse step succeeds (no parse error in stderr).
#[test]
fn pipe_valid_json_parses_without_error() {
    let mut child = Command::new("target/debug/jqt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("JQT_SKIP_TTY_CHECK", "1")
        .spawn()
        .expect("Failed to spawn jqt");

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(br#"{"name":"alice","age":30}"#);
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break,
        }
    }

    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Failed to parse input as JSON"),
        "Valid JSON incorrectly rejected:\n{stderr}"
    );
    assert!(
        !stderr.contains("Failed to initialize input reader"),
        "Crossterm crash with valid JSON:\n{stderr}"
    );
}

/// Invalid JSON must produce a parse error (not a crossterm crash or panic).
#[test]
fn pipe_invalid_json_parse_error() {
    let mut child = Command::new("target/debug/jqt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("JQT_SKIP_TTY_CHECK", "1")
        .spawn()
        .expect("Failed to spawn jqt");

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"this is not json");
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break,
        }
    }

    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to parse input as JSON") || stderr.contains("invalid"),
        "Expected a parse error for invalid JSON, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("Failed to initialize input reader"),
        "Crossterm crash instead of parse error:\n{stderr}"
    );
}
