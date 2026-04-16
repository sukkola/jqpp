//! PTY-based keyboard integration tests.
//!
//! Spawns the binary with a pseudo-terminal as its controlling terminal so
//! crossterm's /dev/tty event reader receives keystrokes injected via the
//! PTY master.
//!
//! Two implementation details are critical:
//!
//! 1. The master side must be drained continuously in a background thread.
//!    If nobody reads master, the slave output buffer fills, terminal.draw()
//!    blocks, and the event loop never calls event::poll().
//!
//! 2. Ctrl+Q is byte 0x11 (XON), which the PTY line discipline silently
//!    discards when IXON is still set (cooked mode).  We must wait until the
//!    app has written its first TUI frame before sending 0x11 — that proves
//!    enable_raw_mode() / cfmakeraw has been called and IXON is cleared.

use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

const BINARY: &str = "target/debug/jqpp";

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

/// Spawn jqpp with the slave PTY as stdin/stdout/stderr and as the
/// controlling terminal (via setsid + TIOCSCTTY).
#[cfg(unix)]
fn spawn_with_pty(slave_fd: libc::c_int) -> std::process::Child {
    use std::os::unix::io::FromRawFd;
    use std::os::unix::process::CommandExt;

    let slave_in = unsafe { libc::dup(slave_fd) };
    let slave_out = unsafe { libc::dup(slave_fd) };
    let slave_err = unsafe { libc::dup(slave_fd) };

    let mut cmd = Command::new(BINARY);
    cmd.stdin(unsafe { Stdio::from_raw_fd(slave_in) })
        .stdout(unsafe { Stdio::from_raw_fd(slave_out) })
        .stderr(unsafe { Stdio::from_raw_fd(slave_err) });

    unsafe {
        cmd.pre_exec(move || {
            libc::setsid();
            libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0i32);
            Ok(())
        });
    }

    cmd.spawn()
        .unwrap_or_else(|_| panic!("Failed to spawn {BINARY} — run `cargo build` first"))
}

/// Drain master in a background thread and signal `ready` when the first
/// TUI bytes appear (= event loop started + cfmakeraw applied).
///
/// Without draining, the slave output buffer fills and terminal.draw() blocks,
/// making the app completely unresponsive to keyboard input.
#[cfg(unix)]
fn start_drain_thread(master_fd: libc::c_int) -> Arc<AtomicBool> {
    let ready = Arc::new(AtomicBool::new(false));
    let ready_clone = ready.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            let n =
                unsafe { libc::read(master_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                break;
            }
            ready_clone.store(true, Ordering::Release);
        }
    });
    ready
}

/// Block until the drain thread signals that the TUI has started rendering,
/// then wait a short extra margin so cfmakeraw settles.
fn wait_for_tui(ready: &Arc<AtomicBool>) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while !ready.load(Ordering::Acquire) {
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Small margin after first frame — raw mode propagation to PTY is sync,
    // but give the event loop one full poll cycle.
    std::thread::sleep(Duration::from_millis(200));
}

/// Write raw bytes to the PTY master (simulates keystrokes).
#[cfg(unix)]
fn pty_write(master_fd: libc::c_int, bytes: &[u8]) {
    unsafe {
        libc::write(
            master_fd,
            bytes.as_ptr() as *const libc::c_void,
            bytes.len(),
        );
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Ctrl+C (byte 0x03) must cause the binary to exit within 3 s.
/// 0x03 is NOT subject to IXON flow-control, so it is safe to send even
/// before cfmakeraw, but we wait for the TUI to start for a clean test.
#[test]
#[cfg(unix)]
fn test_ctrl_c_exits() {
    let (master_fd, slave_fd) = match unsafe { open_pty() } {
        Some(p) => p,
        None => {
            eprintln!("open_pty unavailable — skipping");
            return;
        }
    };

    let mut child = spawn_with_pty(slave_fd);
    unsafe { libc::close(slave_fd) };

    let ready = start_drain_thread(master_fd);
    wait_for_tui(&ready);

    if child.try_wait().ok().flatten().is_some() {
        unsafe { libc::close(master_fd) };
        return;
    }

    pty_write(master_fd, b"\x03"); // Ctrl+C = 0x03

    assert!(
        wait_for_exit(&mut child, Duration::from_secs(3)),
        "Binary did not exit within 3 s after Ctrl+C"
    );
    let _ = child.kill();
    unsafe { libc::close(master_fd) };
}

/// Ctrl+Q (byte 0x11) must cause the binary to exit within 3 s.
/// Byte 0x11 is XON; it must arrive after cfmakeraw clears IXON.
#[test]
#[cfg(unix)]
fn test_ctrl_q_exits() {
    let (master_fd, slave_fd) = match unsafe { open_pty() } {
        Some(p) => p,
        None => {
            eprintln!("open_pty unavailable — skipping");
            return;
        }
    };

    let mut child = spawn_with_pty(slave_fd);
    unsafe { libc::close(slave_fd) };

    let ready = start_drain_thread(master_fd);
    wait_for_tui(&ready); // guarantees IXON is cleared before 0x11 is sent

    if child.try_wait().ok().flatten().is_some() {
        unsafe { libc::close(master_fd) };
        return;
    }

    pty_write(master_fd, b"\x11"); // Ctrl+Q = 0x11

    assert!(
        wait_for_exit(&mut child, Duration::from_secs(3)),
        "Binary did not exit within 3 s after Ctrl+Q"
    );
    let _ = child.kill();
    unsafe { libc::close(master_fd) };
}

/// Tab moves focus to LeftPane; pressing 'q' must then quit.
#[test]
#[cfg(unix)]
fn test_q_after_tab_exits() {
    let (master_fd, slave_fd) = match unsafe { open_pty() } {
        Some(p) => p,
        None => {
            eprintln!("open_pty unavailable — skipping");
            return;
        }
    };

    let mut child = spawn_with_pty(slave_fd);
    unsafe { libc::close(slave_fd) };

    let ready = start_drain_thread(master_fd);
    wait_for_tui(&ready);

    if child.try_wait().ok().flatten().is_some() {
        unsafe { libc::close(master_fd) };
        return;
    }

    pty_write(master_fd, b"\t"); // Tab → LeftPane
    std::thread::sleep(Duration::from_millis(150));
    pty_write(master_fd, b"q"); // q → quit

    assert!(
        wait_for_exit(&mut child, Duration::from_secs(3)),
        "Binary did not exit within 3 s after Tab+q"
    );
    let _ = child.kill();
    unsafe { libc::close(master_fd) };
}

/// Shift+Tab (ESC [ Z) goes to RightPane; 'q' must then quit.
#[test]
#[cfg(unix)]
fn test_q_after_shift_tab_exits() {
    let (master_fd, slave_fd) = match unsafe { open_pty() } {
        Some(p) => p,
        None => {
            eprintln!("open_pty unavailable — skipping");
            return;
        }
    };

    let mut child = spawn_with_pty(slave_fd);
    unsafe { libc::close(slave_fd) };

    let ready = start_drain_thread(master_fd);
    wait_for_tui(&ready);

    if child.try_wait().ok().flatten().is_some() {
        unsafe { libc::close(master_fd) };
        return;
    }

    pty_write(master_fd, b"\x1b[Z"); // Shift+Tab → RightPane
    std::thread::sleep(Duration::from_millis(150));
    pty_write(master_fd, b"q"); // q → quit

    assert!(
        wait_for_exit(&mut child, Duration::from_secs(3)),
        "Binary did not exit within 3 s after Shift+Tab+q"
    );
    let _ = child.kill();
    unsafe { libc::close(master_fd) };
}
