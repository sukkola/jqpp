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
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

const BINARY: &str = "target/debug/jqpp";
const SAMPLE_JSON: &[u8] = br#"{"name":"test","value":42,"nested":{"x":1}}"#;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Open a pseudo-terminal pair → (master_fd, slave_fd).
#[cfg(unix)]
unsafe fn open_pty() -> Option<(libc::c_int, libc::c_int)> {
    unsafe {
        let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
        if master < 0 {
            return None;
        }
        if libc::grantpt(master) != 0 {
            libc::close(master);
            return None;
        }
        if libc::unlockpt(master) != 0 {
            libc::close(master);
            return None;
        }
        let slave_ptr = libc::ptsname(master);
        if slave_ptr.is_null() {
            libc::close(master);
            return None;
        }
        let slave_path =
            std::ffi::CString::new(std::ffi::CStr::from_ptr(slave_ptr).to_bytes()).ok()?;
        let slave = libc::open(slave_path.as_ptr(), libc::O_RDWR);
        if slave < 0 {
            libc::close(master);
            return None;
        }
        Some((master, slave))
    }
}

#[cfg(unix)]
struct PtyCapture {
    master_fd: libc::c_int,
    ready: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<u8>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

#[cfg(unix)]
impl PtyCapture {
    fn start(master_fd: libc::c_int) -> Self {
        let ready = Arc::new(AtomicBool::new(false));
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let ready_clone = ready.clone();
        let buffer_clone = buffer.clone();
        let handle = std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            loop {
                let n = unsafe {
                    libc::read(
                        master_fd,
                        chunk.as_mut_ptr() as *mut libc::c_void,
                        chunk.len(),
                    )
                };
                if n <= 0 {
                    break;
                }
                ready_clone.store(true, Ordering::Release);
                buffer_clone
                    .lock()
                    .expect("capture buffer poisoned")
                    .extend_from_slice(&chunk[..n as usize]);
            }
        });
        Self {
            master_fd,
            ready,
            buffer,
            handle: Some(handle),
        }
    }

    fn wait_for_tui(&self) {
        let deadline = Instant::now() + Duration::from_secs(4);
        while !self.ready.load(Ordering::Acquire) {
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    fn finish(mut self) -> String {
        unsafe { libc::close(self.master_fd) };
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        String::from_utf8_lossy(&self.buffer.lock().expect("capture buffer poisoned").clone())
            .into_owned()
    }
}

/// Spawn the binary, optionally send bytes to stdin, then wait for it to exit
/// on its own up to `wait_before_signal`.  Returns the child plus an isolated
/// PTY capture so the app can open `/dev/tty` without touching the real test terminal.
#[cfg(unix)]
fn spawn_with_input(input: &[u8]) -> (std::process::Child, PtyCapture) {
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::CommandExt;

    let (master_fd, slave_fd) =
        unsafe { open_pty() }.unwrap_or_else(|| panic!("Failed to open PTY for test"));
    let slave_out = unsafe { libc::dup(slave_fd) };
    let slave_err = unsafe { libc::dup(slave_fd) };

    let mut command = Command::new(BINARY);
    command
        .stdin(Stdio::piped())
        .stdout(unsafe { Stdio::from_raw_fd(slave_out) })
        .stderr(unsafe { Stdio::from_raw_fd(slave_err) });

    unsafe {
        command.pre_exec(move || {
            libc::setsid();
            libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0i32);
            Ok(())
        });
    }

    let mut child = command
        .spawn()
        .unwrap_or_else(|_| panic!("Failed to spawn {BINARY} — run `cargo build` first"));

    unsafe { libc::close(slave_fd) };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input);
        // Close stdin so the binary can proceed past stdin read
    }

    let capture = PtyCapture::start(master_fd);
    capture.wait_for_tui();
    (child, capture)
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
    let (mut child, capture) = spawn_with_input(SAMPLE_JSON);

    // If it already exited (headless env / no TTY), we're done.
    if child.try_wait().ok().flatten().is_some() {
        let _ = capture.finish();
        return;
    }

    // Binary is still running — send SIGTERM and expect it to exit promptly.
    send_signal(&child, libc::SIGTERM);

    assert!(
        wait_for_exit(&mut child, Duration::from_secs(2)),
        "Binary did not exit within 2 s after SIGTERM — event loop may be stuck"
    );

    let _ = child.kill();
    let _ = capture.finish();
}

// ── SIGINT ────────────────────────────────────────────────────────────────────

/// SIGINT (what a user sends via `kill -INT <pid>` when the app is stuck) must
/// also terminate the binary promptly.
#[test]
#[cfg(unix)]
fn test_sigint_exits_within_deadline() {
    let (mut child, capture) = spawn_with_input(SAMPLE_JSON);

    if child.try_wait().ok().flatten().is_some() {
        let _ = capture.finish();
        return;
    }

    send_signal(&child, libc::SIGINT);

    assert!(
        wait_for_exit(&mut child, Duration::from_secs(2)),
        "Binary did not exit within 2 s after SIGINT — signal handler may not be installed"
    );

    let _ = child.kill();
    let _ = capture.finish();
}

// ── No hang on rapid key sequences (regression for clipboard blocking) ───────

/// Start the binary, give it 200 ms, then forcibly kill it.  The binary must
/// not be in a wedged state (blocking on clipboard / arboard) — it must die
/// immediately on SIGKILL.  SIGKILL is always delivered so this really just
/// verifies the process is alive and killable, not stuck in a kernel trap.
#[test]
#[cfg(unix)]
fn test_process_is_always_killable() {
    let (mut child, capture) = spawn_with_input(SAMPLE_JSON);

    // Kill it hard — must succeed immediately.
    let _ = child.kill();
    assert!(
        wait_for_exit(&mut child, Duration::from_millis(500)),
        "Binary did not die after SIGKILL — process is in an unkillable state"
    );
    let _ = capture.finish();
}

// ── Does not emit internal crossterm crash on rapid re-launch ─────────────────

/// Rapidly launch and kill the binary several times in a row.  None of the runs
/// should produce the crossterm "Failed to initialize input reader" crash.
/// This is a regression guard: if we accidentally remove the `use-dev-tty`
/// feature, this test catches it.
#[test]
fn test_no_crash_on_repeated_start() {
    for _ in 0..3 {
        let (mut child, capture) = spawn_with_input(SAMPLE_JSON);
        let _ = child.kill();
        let _ = child.wait();
        let stderr = capture.finish();
        assert!(
            !stderr.contains("Failed to initialize input reader"),
            "Crossterm internal crash on run:\n{stderr}"
        );
    }
}
