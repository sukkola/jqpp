use anyhow::{Context, Result};
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableFocusChange,
    EnableMouseCapture,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode};
use std::io::{self, Write};

pub struct TtyWriter(pub std::fs::File);

impl Write for TtyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

pub struct TerminalGuard {
    pub tty_handle: Option<std::fs::File>,
}

impl TerminalGuard {
    pub fn create(tty: Option<&std::fs::File>) -> Result<Self> {
        ratatui::crossterm::terminal::enable_raw_mode().context("Failed to enable raw mode")?;

        let tty_clone = tty.and_then(|t| t.try_clone().ok());

        let setup_result = if let Some(tty_handle) = tty {
            let mut writer = TtyWriter(
                tty_handle
                    .try_clone()
                    .context("Failed to clone TTY handle for writer")?,
            );
            execute!(
                writer,
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableFocusChange,
                EnableBracketedPaste,
                Hide
            )
            .context("Failed to setup TTY terminal state")
        } else {
            execute!(
                io::stdout(),
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableFocusChange,
                EnableBracketedPaste,
                Hide
            )
            .context("Failed to initialize terminal state")
        };

        if let Err(e) = setup_result {
            let _ = ratatui::crossterm::terminal::disable_raw_mode();
            return Err(e);
        }

        Ok(Self {
            tty_handle: tty_clone,
        })
    }
}

impl Drop for TerminalGuard {
    #[allow(clippy::collapsible_if)]
    fn drop(&mut self) {
        let _ = disable_raw_mode();

        if let Some(ref tty) = self.tty_handle {
            if let Ok(cloned) = tty.try_clone() {
                let mut writer = TtyWriter(cloned);
                let _ = execute!(
                    writer,
                    DisableBracketedPaste,
                    LeaveAlternateScreen,
                    DisableMouseCapture,
                    Show
                );
                return;
            }
        }

        #[cfg(unix)]
        {
            if let Ok(tty) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
            {
                let mut writer = TtyWriter(tty);
                let _ = execute!(
                    writer,
                    DisableBracketedPaste,
                    LeaveAlternateScreen,
                    DisableMouseCapture,
                    Show
                );
                return;
            }
        }
        let mut stdout = io::stdout();
        let _ = execute!(
            stdout,
            DisableBracketedPaste,
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show
        );
    }
}

pub fn setup_panic_hook(debug: bool) {
    let original_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        #[cfg(unix)]
        {
            if let Ok(tty) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
            {
                let mut writer = TtyWriter(tty);
                let _ = execute!(writer, LeaveAlternateScreen, DisableMouseCapture, Show);
            } else {
                let _ = execute!(
                    io::stdout(),
                    LeaveAlternateScreen,
                    DisableMouseCapture,
                    Show
                );
            }
        }
        #[cfg(not(unix))]
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show
        );

        if debug {
            original_panic_hook(panic_info);
        } else {
            eprintln!("jqpp panicked. Use --debug for more info.");
        }
    }));
}

pub fn lsp_on_path() -> bool {
    let bin = std::env::var("JQPP_LSP_BIN").unwrap_or_else(|_| "jq-lsp".to_string());
    let path = std::path::Path::new(&bin);
    if path.is_absolute() {
        return path.is_file();
    }
    std::env::var("PATH")
        .map(|p| std::env::split_paths(&p).any(|dir| dir.join(&bin).is_file()))
        .unwrap_or(false)
}

pub fn get_tty_handle() -> Option<std::fs::File> {
    #[cfg(unix)]
    {
        if let Ok(tty) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
        {
            return Some(tty);
        }

        for fd in [libc::STDOUT_FILENO, libc::STDERR_FILENO, libc::STDIN_FILENO] {
            if unsafe { libc::isatty(fd) } != 0 {
                let ptr = unsafe { libc::ttyname(fd) };
                if !ptr.is_null() {
                    let path = unsafe { std::ffi::CStr::from_ptr(ptr) }
                        .to_string_lossy()
                        .to_string();
                    if let Ok(tty) = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&path)
                    {
                        return Some(tty);
                    }
                }
            }
        }
    }
    None
}
