use jqpp::app::{App, AppState, DragTarget};
use jqpp::completions;
use jqpp::completions::lsp::{LspMessage, LspProvider};
use jqpp::config;
use jqpp::executor::Executor;
use jqpp::keymap;
use jqpp::ui;
use jqpp::widgets;

use anyhow::{Context, Result};
#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
))]
use arboard::SetExtLinux;
use clap::Parser;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableFocusChange,
    EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Positional [file]
    file: Option<PathBuf>,

    /// Disable LSP even if jq-lsp is found on PATH
    #[arg(long)]
    no_lsp: bool,

    /// Enable debug mode (shows stack traces)
    #[arg(long)]
    debug: bool,

    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Print effective config and exit
    #[arg(long)]
    print_config: bool,
}

struct TtyWriter(std::fs::File);
impl Write for TtyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

struct TerminalGuard {
    tty_handle: Option<std::fs::File>,
}

impl TerminalGuard {
    fn create(tty: Option<&std::fs::File>) -> Result<Self> {
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

fn setup_panic_hook(debug: bool) {
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

fn main() {
    let args = Args::parse();
    if args.debug {
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
    }

    if let Err(e) = actual_main(args) {
        if std::env::var("RUST_BACKTRACE").is_ok() {
            eprintln!("jqpp CRITICAL ERROR: {:?}", e);
        } else {
            eprintln!("jqpp CRITICAL ERROR: {}", e);
            eprintln!("\nRun with --debug to see a full stack trace.");
        }
        std::process::exit(1);
    }
}

fn lsp_on_path() -> bool {
    let bin = std::env::var("JQPP_LSP_BIN").unwrap_or_else(|_| "jq-lsp".to_string());
    let path = std::path::Path::new(&bin);
    if path.is_absolute() {
        return path.is_file();
    }
    std::env::var("PATH")
        .map(|p| std::env::split_paths(&p).any(|dir| dir.join(&bin).is_file()))
        .unwrap_or(false)
}

fn get_tty_handle() -> Option<std::fs::File> {
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

fn copy_text_to_clipboard(text: String) {
    std::thread::spawn(move || {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return;
        };

        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
        ))]
        {
            let deadline = Instant::now() + Duration::from_millis(250);
            let _ = clipboard.set().wait_until(deadline).text(text);
        }

        #[cfg(not(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
        )))]
        {
            let _ = clipboard.set_text(text);
        }
    });
}

fn actual_main(args: Args) -> Result<()> {
    setup_panic_hook(args.debug);

    let mut input_data = Vec::new();
    let stdin_is_terminal = io::stdin().is_terminal();

    if let Some(ref f_path) = args.file {
        input_data = std::fs::read(f_path).context(format!("Failed to read file: {:?}", f_path))?;
    } else if !stdin_is_terminal {
        io::stdin()
            .read_to_end(&mut input_data)
            .context("Failed to read from stdin pipe")?;
    }

    let tty_handle = get_tty_handle();

    use std::os::unix::io::AsRawFd;
    if let Some(ref tty) = tty_handle
        && !stdin_is_terminal
    {
        unsafe {
            if libc::dup2(tty.as_raw_fd(), libc::STDIN_FILENO) == -1 {
                return Err(anyhow::anyhow!("Failed to redirect TTY to stdin"));
            }
        }
    }

    if !stdin_is_terminal && tty_handle.is_none() && std::env::var("JQPP_SKIP_TTY_CHECK").is_err() {
        return Err(anyhow::anyhow!(
            "No TTY found for interactive mode while stdin is redirected."
        ));
    }

    let executor = if !input_data.is_empty() {
        let json_input: serde_json::Value =
            serde_json::from_slice(&input_data).context("Failed to parse input as JSON")?;
        Some(Executor {
            raw_input: input_data,
            json_input,
            source_label: args
                .file
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "stdin".to_string()),
        })
    } else {
        None
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let (keymap, config_error) = config::load_keymap(args.config.as_deref());

    if args.print_config {
        let mut keys = std::collections::HashMap::new();
        for (action, binding) in keymap.0 {
            keys.insert(action.toml_name().to_string(), binding.to_string());
        }
        let config = config::Config { keys };
        println!("{}", toml::to_string(&config)?);
        return Ok(());
    }

    rt.block_on(run(executor, args, tty_handle, keymap, config_error))
}

async fn run(
    executor: Option<Executor>,
    args: Args,
    tty_handle: Option<std::fs::File>,
    keymap: keymap::Keymap,
    config_error: Option<String>,
) -> Result<()> {
    // Headless mode: used by integration tests. Start LSP if requested but
    // never touch the terminal — no raw mode, no alternate screen.
    let use_lsp = !args.no_lsp && lsp_on_path();

    if std::env::var("JQPP_SKIP_TTY_CHECK").is_ok() {
        if use_lsp {
            let (lsp_tx, _lsp_rx) = mpsc::channel::<LspMessage>(100);
            let mut provider = LspProvider::new();
            let _ = provider.start(lsp_tx).await;
            // Park until the test kills us.
            tokio::time::sleep(Duration::from_secs(60)).await;
            let _ = provider.shutdown().await;
        }
        return Ok(());
    }

    let mut app = App::new();
    app.lsp_enabled = use_lsp;
    app.executor = executor;

    if let Some(err) = config_error {
        app.footer_message = Some(err);
        app.footer_message_at = Some(Instant::now());
    }

    let (lsp_tx, mut lsp_rx) = mpsc::channel::<LspMessage>(100);
    let lsp_provider = if use_lsp {
        let mut provider = LspProvider::new();
        if provider.start(lsp_tx).await.is_ok() {
            Some(provider)
        } else {
            app.lsp_status = Some("LSP initializing...".to_string());
            None
        }
    } else {
        None
    };

    let _guard = TerminalGuard::create(
        tty_handle
            .as_ref()
            .and_then(|f| f.try_clone().ok())
            .as_ref(),
    )?;

    match tty_handle {
        Some(tty) => {
            let backend = CrosstermBackend::new(TtyWriter(tty));
            let mut terminal = Terminal::new(backend)?;
            main_loop(&mut terminal, &mut app, lsp_provider, &mut lsp_rx, &keymap).await
        }
        None => {
            let backend = CrosstermBackend::new(io::stdout());
            let mut terminal = Terminal::new(backend)?;
            main_loop(&mut terminal, &mut app, lsp_provider, &mut lsp_rx, &keymap).await
        }
    }
}

async fn main_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App<'_>,
    lsp_provider: Option<LspProvider>,
    lsp_rx: &mut mpsc::Receiver<LspMessage>,
    keymap: &keymap::Keymap,
) -> Result<()> {
    let debounce_duration = Duration::from_millis(80);
    let mut last_edit_at = Instant::now();
    let mut debounce_pending = false;

    let mut key_log: Option<std::fs::File> = std::env::var("JQPP_KEY_LOG").ok().and_then(|path| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
    });

    if let Some(ref exec) = app.executor {
        let input = exec.json_input.clone();
        if let Ok(Ok(results)) =
            tokio::task::spawn_blocking(move || Executor::execute(".", &input)).await
        {
            app.results = results;
        }
    }

    let mut footer_message: Option<(String, Instant)> =
        if let (Some(m), Some(at)) = (app.footer_message.take(), app.footer_message_at.take()) {
            Some((m, at))
        } else {
            None
        };
    let mut lsp_completions: Vec<completions::CompletionItem> = Vec::new();
    let mut suggestion_active = false;
    let mut cached_pipe_type: Option<String> = None;
    let mut last_esc_at: Option<Instant> = None;
    let mut discard_sgr_mouse_until: Option<Instant> = None;
    let mut suppress_scroll_until: Option<Instant> = None;
    let mut drop_scroll_backlog_until: Option<Instant> = None;
    type ComputeResult = (
        anyhow::Result<(Vec<serde_json::Value>, bool)>,
        Option<String>,
    );
    let mut compute_handle: Option<tokio::task::JoinHandle<ComputeResult>> = None;
    let mut pending_qp: String = String::new();

    while app.running {
        #[allow(clippy::collapsible_if)]
        if let Some(ref handle) = compute_handle {
            if handle.is_finished() {
                match compute_handle.take().unwrap().await {
                    Ok((Ok((results, raw)), pipe_type)) => {
                        app.results = results;
                        app.right_scroll = 0;
                        app.error = None;
                        app.raw_output = raw;
                        cached_pipe_type = pipe_type;
                    }
                    Ok((Err(_), pipe_type)) => {
                        app.raw_output = false;
                        cached_pipe_type = pipe_type;
                    }
                    Err(_) => {}
                }
                if suggestion_active {
                    app.query_input.suggestions = compute_suggestions(
                        &pending_qp,
                        app.executor.as_ref().map(|e| &e.json_input),
                        &lsp_completions,
                        cached_pipe_type.as_deref(),
                    );
                    app.query_input.suggestion_index = 0;
                    app.query_input.suggestion_scroll = 0;
                    let all_exact = !app.query_input.suggestions.is_empty()
                        && app
                            .query_input
                            .suggestions
                            .iter()
                            .all(|s| s.insert_text == pending_qp);
                    if all_exact {
                        app.query_input.show_suggestions = false;
                        suggestion_active = false;
                        lsp_completions.clear();
                        cached_pipe_type = None;
                    } else {
                        app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
                    }
                }
            }
        }

        terminal
            .draw(|f| ui::draw(f, app, keymap))
            .context("Failed to draw TUI frame")?;

        while let Ok(msg) = lsp_rx.try_recv() {
            match msg {
                LspMessage::Status(s) => {
                    app.lsp_status = if s == "ready" { None } else { Some(s) };
                }
                LspMessage::Diagnostic(d) => {
                    app.lsp_diagnostic = d;
                }
                LspMessage::Completions(c) => {
                    if !c.is_empty() {
                        lsp_completions = c;
                    }
                    if suggestion_active {
                        let query_line = app.query_input.textarea.lines()[0].clone();
                        let cur = app.query_input.textarea.cursor().1;
                        let query_prefix: String = query_line.chars().take(cur).collect();
                        app.query_input.suggestions = compute_suggestions(
                            &query_prefix,
                            app.executor.as_ref().map(|e| &e.json_input),
                            &lsp_completions,
                            cached_pipe_type.as_deref(),
                        );
                        app.query_input.suggestion_index = 0;
                        app.query_input.suggestion_scroll = 0;
                        let all_exact = !app.query_input.suggestions.is_empty()
                            && app
                                .query_input
                                .suggestions
                                .iter()
                                .all(|s| s.insert_text == query_prefix);
                        if all_exact {
                            app.query_input.show_suggestions = false;
                            suggestion_active = false;
                            lsp_completions.clear();
                            cached_pipe_type = None;
                        } else {
                            app.query_input.show_suggestions =
                                !app.query_input.suggestions.is_empty();
                        }
                    }
                }
            }
        }

        if event::poll(Duration::from_millis(8)).context("Failed to poll for terminal events")? {
            const SCROLL_CIRCUIT_THRESHOLD: usize = 64;
            const SCROLL_CIRCUIT_MS: u64 = 120;
            const MAX_PENDING_EVENTS: usize = 256;
            const MAX_READ_EVENTS_NORMAL: usize = 512;
            const MAX_READ_EVENTS_WHILE_SUPPRESSED: usize = 4096;
            const BACKLOG_DROP_MS: u64 = 220;

            if suppress_scroll_until
                .map(|until| Instant::now() > until)
                .unwrap_or(false)
            {
                suppress_scroll_until = None;
            }
            if drop_scroll_backlog_until
                .map(|until| Instant::now() > until)
                .unwrap_or(false)
            {
                drop_scroll_backlog_until = None;
            }

            let now = Instant::now();
            let mut pending_events = Vec::with_capacity(48);
            let mut queued_left_scroll = app.left_scroll;
            let mut queued_right_scroll = app.right_scroll;
            let mut latest_scroll: Option<(ScrollPane, i16)> = None;
            let mut queued_scroll_events = 0usize;
            let mut drained_reads = 0usize;
            let read_budget =
                if scroll_input_suppressed(suppress_scroll_until, drop_scroll_backlog_until) {
                    MAX_READ_EVENTS_WHILE_SUPPRESSED
                } else {
                    MAX_READ_EVENTS_NORMAL
                };

            let first_event = match event::read() {
                Ok(evt) => evt,
                Err(_) => continue,
            };

            let mut queue_event = |evt: Event, pending_events: &mut Vec<Event>| {
                if is_scroll_event(&evt) {
                    if scroll_input_suppressed(suppress_scroll_until, drop_scroll_backlog_until) {
                        return;
                    }
                    queued_scroll_events += 1;
                    if queued_scroll_events > SCROLL_CIRCUIT_THRESHOLD {
                        suppress_scroll_until =
                            Some(now + Duration::from_millis(SCROLL_CIRCUIT_MS));
                        return;
                    }
                }

                if let Event::Mouse(mouse) = &evt
                    && let Some((pane, dir)) = mouse_scroll_direction(app, mouse)
                {
                    let delta = if dir > 0 { 1 } else { -1 };
                    latest_scroll = match latest_scroll {
                        Some((current_pane, current_delta)) if current_pane == pane => {
                            Some((pane, (current_delta + delta).clamp(-24, 24)))
                        }
                        _ => Some((pane, delta)),
                    };

                    let (virt_scroll, max_scroll) = match pane {
                        ScrollPane::Left => (&mut queued_left_scroll, app.max_left_scroll()),
                        ScrollPane::Right => (&mut queued_right_scroll, app.max_right_scroll()),
                    };
                    let can_scroll = if dir > 0 {
                        *virt_scroll < max_scroll
                    } else {
                        *virt_scroll > 0
                    };
                    if !can_scroll {
                        return;
                    }
                    if dir > 0 {
                        *virt_scroll = virt_scroll.saturating_add(1).min(max_scroll);
                    } else {
                        *virt_scroll = virt_scroll.saturating_sub(1);
                    }

                    return;
                }

                if should_drop_boundary_scroll_event(app, &evt) {
                    return;
                }

                if pending_events.len() < MAX_PENDING_EVENTS {
                    pending_events.push(evt);
                }
            };

            queue_event(first_event, &mut pending_events);
            drained_reads += 1;

            while event::poll(Duration::from_millis(0))
                .context("Failed to poll for pending terminal events")?
            {
                if drained_reads >= read_budget {
                    drop_scroll_backlog_until = Some(now + Duration::from_millis(BACKLOG_DROP_MS));
                    break;
                }
                let evt = match event::read() {
                    Ok(evt) => evt,
                    Err(_) => break,
                };
                queue_event(evt, &mut pending_events);
                drained_reads += 1;
            }

            let scroll_boost: i16 = if pending_events.len() >= 96 {
                8
            } else if pending_events.len() >= 48 {
                4
            } else if pending_events.len() >= 16 {
                2
            } else {
                1
            };

            for event in pending_events {
                match event {
                    Event::FocusGained => {
                        terminal.clear().ok();
                    }
                    Event::Key(key) => {
                        if matches!(app.state, AppState::QueryInput) {
                            if key.code == KeyCode::Esc {
                                discard_sgr_mouse_until =
                                    Some(Instant::now() + Duration::from_millis(60));
                            } else if let Some(deadline) = discard_sgr_mouse_until {
                                if Instant::now() <= deadline
                                    && let KeyCode::Char(c) = key.code
                                    && matches!(c, '[' | '<' | ';' | 'M' | 'm' | '0'..='9')
                                {
                                    if matches!(c, 'M' | 'm') {
                                        discard_sgr_mouse_until = None;
                                    }
                                    continue;
                                }
                                discard_sgr_mouse_until = None;
                            }
                        }

                        if let Some(ref mut log) = key_log {
                            let _ = writeln!(
                                &*log,
                                "key: {:?} mods: {:?} kind: {:?}",
                                key.code, key.modifiers, key.kind
                            );
                        }

                        let is_action = |a: keymap::Action| keymap.is_action(a, &key);

                        let is_ctrl_quit = is_action(keymap::Action::Quit)
                            || (key.modifiers.contains(KeyModifiers::CONTROL)
                                && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')));
                        let is_pane_quit = !matches!(app.state, AppState::QueryInput)
                            && (matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
                                || key.code == KeyCode::Esc);

                        if is_ctrl_quit || is_pane_quit {
                            app.running = false;
                            continue;
                        }

                        let is_copy = is_action(keymap::Action::CopyClipboard)
                            || (key.modifiers.contains(KeyModifiers::SUPER)
                                && key.code == KeyCode::Char('c'));
                        if is_copy {
                            let text = match app.state {
                                AppState::QueryInput => {
                                    Some(app.query_input.textarea.lines()[0].clone())
                                }
                                AppState::LeftPane => app
                                    .executor
                                    .as_ref()
                                    .map(|e| String::from_utf8_lossy(&e.raw_input).into_owned()),
                                AppState::RightPane => {
                                    if let Some(ref err) = app.error {
                                        Some(err.clone())
                                    } else {
                                        Some(Executor::format_results(&app.results, app.raw_output))
                                    }
                                }
                                AppState::SideMenu => None,
                            };
                            if let Some(t) = text {
                                copy_text_to_clipboard(t);
                                footer_message = Some(("copied".to_string(), Instant::now()));
                            }
                            continue;
                        }

                        match app.state {
                            AppState::QueryInput => {
                                if is_action(keymap::Action::Submit) {
                                    if app.query_input.show_suggestions
                                        && !app.query_input.suggestions.is_empty()
                                    {
                                        let suggestion = app.query_input.suggestions
                                            [app.query_input.suggestion_index]
                                            .insert_text
                                            .clone();
                                        let cur = app.query_input.textarea.cursor().1;
                                        let full = app.query_input.textarea.lines()[0].clone();
                                        let suffix: String = full.chars().skip(cur).collect();
                                        let new_text = format!("{}{}", suggestion, suffix);
                                        let col = cursor_col_after_accept(&suggestion);
                                        app.query_input.textarea =
                                            tui_textarea::TextArea::from(vec![new_text]);
                                        app.query_input.textarea.set_block(
                                            ratatui::widgets::Block::default()
                                                .title(" Query ")
                                                .borders(ratatui::widgets::Borders::ALL),
                                        );
                                        app.query_input
                                        .textarea
                                        .set_cursor_line_style(ratatui::style::Style::default());
                                        app.query_input
                                            .textarea
                                            .move_cursor(tui_textarea::CursorMove::Jump(0, col));
                                        app.query_input.show_suggestions = false;
                                        suggestion_active = false;
                                        lsp_completions.clear();
                                        cached_pipe_type = None;
                                        last_edit_at = Instant::now() - debounce_duration;
                                        debounce_pending = true;
                                    } else {
                                        app.query_input.show_suggestions = false;
                                        suggestion_active = false;
                                        let query = app.query_input.textarea.lines()[0].clone();
                                        app.query_input.push_history(query.clone());
                                        if let Some(ref exec) = app.executor {
                                            match Executor::execute_query(&query, &exec.json_input)
                                            {
                                                Ok((results, raw)) => {
                                                    app.results = results;
                                                    app.error = None;
                                                    app.raw_output = raw;
                                                }
                                                Err(e) => {
                                                    app.error = Some(e.to_string());
                                                    app.results = Vec::new();
                                                    app.raw_output = false;
                                                }
                                            }
                                        }
                                    }
                                } else if is_action(keymap::Action::SaveOutput) {
                                    let output =
                                        Executor::format_results(&app.results, app.raw_output);
                                    if std::fs::write("jqpp-output.json", output).is_ok() {
                                        footer_message =
                                            Some(("saved".to_string(), Instant::now()));
                                    }
                                } else if is_action(keymap::Action::AcceptSuggestion)
                                    || is_action(keymap::Action::NextPane)
                                {
                                    if app.query_input.show_suggestions
                                        && !app.query_input.suggestions.is_empty()
                                        && is_action(keymap::Action::AcceptSuggestion)
                                    {
                                        let suggestion = app.query_input.suggestions
                                            [app.query_input.suggestion_index]
                                            .insert_text
                                            .clone();
                                        let cur = app.query_input.textarea.cursor().1;
                                        let full = app.query_input.textarea.lines()[0].clone();
                                        let suffix: String = full.chars().skip(cur).collect();
                                        let new_text = format!("{}{}", suggestion, suffix);
                                        let col = cursor_col_after_accept(&suggestion);
                                        app.query_input.textarea =
                                            tui_textarea::TextArea::from(vec![new_text]);
                                        app.query_input.textarea.set_block(
                                            ratatui::widgets::Block::default()
                                                .title(" Query ")
                                                .borders(ratatui::widgets::Borders::ALL),
                                        );
                                        app.query_input
                                        .textarea
                                        .set_cursor_line_style(ratatui::style::Style::default());
                                        app.query_input
                                            .textarea
                                            .move_cursor(tui_textarea::CursorMove::Jump(0, col));
                                        app.query_input.show_suggestions = false;
                                        suggestion_active = false;
                                        lsp_completions.clear();
                                        cached_pipe_type = None;
                                        last_edit_at = Instant::now() - debounce_duration;
                                        debounce_pending = true;
                                    } else if is_action(keymap::Action::NextPane) {
                                        app.next_pane();
                                    }
                                } else if is_action(keymap::Action::PrevPane) {
                                    app.query_input.show_suggestions = false;
                                    suggestion_active = false;
                                    app.prev_pane();
                                } else if is_action(keymap::Action::SuggestionUp)
                                    || is_action(keymap::Action::HistoryUp)
                                {
                                    if app.query_input.show_suggestions
                                        && is_action(keymap::Action::SuggestionUp)
                                    {
                                        if app.query_input.suggestion_index > 0 {
                                            app.query_input.suggestion_index -= 1;
                                            app.query_input.clamp_scroll();
                                        } else {
                                            app.query_input.show_suggestions = false;
                                            suggestion_active = false;
                                            lsp_completions.clear();
                                            cached_pipe_type = None;
                                        }
                                    } else if is_action(keymap::Action::HistoryUp) {
                                        if suggestion_active
                                            && !app.query_input.suggestions.is_empty()
                                        {
                                            app.query_input.show_suggestions = true;
                                            app.query_input.suggestion_index =
                                                app.query_input.suggestions.len().saturating_sub(1);
                                            app.query_input.clamp_scroll();
                                        } else {
                                            app.query_input.history_up();
                                        }
                                    }
                                } else if is_action(keymap::Action::SuggestionDown)
                                    || is_action(keymap::Action::HistoryDown)
                                {
                                    if app.query_input.show_suggestions
                                        && is_action(keymap::Action::SuggestionDown)
                                    {
                                        if app.query_input.suggestion_index + 1
                                            < app.query_input.suggestions.len()
                                        {
                                            app.query_input.suggestion_index += 1;
                                            app.query_input.clamp_scroll();
                                        }
                                    } else if is_action(keymap::Action::HistoryDown)
                                        || is_action(keymap::Action::SuggestionDown)
                                    {
                                        suggestion_active = true;
                                        if !app.query_input.suggestions.is_empty() {
                                            app.query_input.show_suggestions = true;
                                            app.query_input.suggestion_index = 0;
                                            app.query_input.clamp_scroll();
                                        } else {
                                            last_edit_at = Instant::now() - debounce_duration;
                                            debounce_pending = true;
                                        }
                                    }
                                } else if key.code == KeyCode::Esc {
                                    if app.query_input.show_suggestions {
                                        app.query_input.show_suggestions = false;
                                        suggestion_active = false;
                                        lsp_completions.clear();
                                        cached_pipe_type = None;
                                        last_esc_at = Some(Instant::now());
                                    } else if last_esc_at
                                        .map(|t| t.elapsed() < Duration::from_millis(500))
                                        .unwrap_or(false)
                                    {
                                        let mut new_ta = tui_textarea::TextArea::default();
                                        new_ta.set_block(
                                            ratatui::widgets::Block::default()
                                                .title(" Query ")
                                                .borders(ratatui::widgets::Borders::ALL),
                                        );
                                        new_ta.set_cursor_line_style(ratatui::style::Style::default());
                                        app.query_input.textarea = new_ta;
                                        app.query_input.show_suggestions = false;
                                        suggestion_active = false;
                                        lsp_completions.clear();
                                        cached_pipe_type = None;
                                        last_esc_at = None;
                                        last_edit_at = Instant::now() - debounce_duration;
                                        debounce_pending = true;
                                    } else {
                                        last_esc_at = Some(Instant::now());
                                    }
                                } else if is_action(keymap::Action::ToggleQueryBar) {
                                    app.query_bar_visible = !app.query_bar_visible;
                                    if !app.query_bar_visible {
                                        app.state = AppState::LeftPane;
                                    }
                                } else if is_action(keymap::Action::ToggleMenu) {
                                    app.side_menu.visible = !app.side_menu.visible;
                                    if app.side_menu.visible {
                                        app.state = AppState::SideMenu;
                                    } else if matches!(app.state, AppState::SideMenu) {
                                        app.state = AppState::QueryInput;
                                    }
                                } else if !should_ignore_query_input_key(&key)
                                    && app.query_input.textarea.input(key)
                                {
                                    last_edit_at = Instant::now();
                                    debounce_pending = true;
                                    match key.code {
                                        KeyCode::Char('.')
                                        | KeyCode::Char('|')
                                        | KeyCode::Char('{')
                                        | KeyCode::Char('[')
                                        | KeyCode::Char(',')
                                        | KeyCode::Char('@') => {
                                            suggestion_active = true;
                                        }
                                        KeyCode::Char(c)
                                            if c.is_alphanumeric()
                                                || c == '_'
                                                || c == '-'
                                                || c == ' ' => {}
                                        KeyCode::Backspace | KeyCode::Delete => {
                                            suggestion_active = true;
                                        }
                                        _ => {
                                            suggestion_active = false;
                                            app.query_input.show_suggestions = false;
                                        }
                                    }
                                }
                            }
                            AppState::SideMenu => {
                                if is_action(keymap::Action::NextPane) {
                                    app.next_pane();
                                } else if is_action(keymap::Action::PrevPane) {
                                    app.prev_pane();
                                } else if is_action(keymap::Action::SuggestionUp) {
                                    if app.side_menu.selected > 0 {
                                        app.side_menu.selected -= 1;
                                    } else {
                                        app.side_menu.selected = app.side_menu.items.len() - 1;
                                    }
                                } else if is_action(keymap::Action::SuggestionDown) {
                                    if app.side_menu.selected + 1 < app.side_menu.items.len() {
                                        app.side_menu.selected += 1;
                                    } else {
                                        app.side_menu.selected = 0;
                                    }
                                } else if is_action(keymap::Action::ToggleMenu) {
                                    app.side_menu.visible = false;
                                    app.state = AppState::QueryInput;
                                }
                            }
                            _ => {
                                if is_action(keymap::Action::NextPane) {
                                    app.next_pane();
                                } else if is_action(keymap::Action::PrevPane) {
                                    app.prev_pane();
                                } else if is_action(keymap::Action::ScrollDown)
                                    || matches!(key.code, KeyCode::Down)
                                {
                                    let (scroll, pane_height, content_lines) =
                                        if matches!(app.state, AppState::LeftPane) {
                                            (
                                                &mut app.left_scroll,
                                                app.left_pane_height,
                                                app.left_content_lines,
                                            )
                                        } else {
                                            (
                                                &mut app.right_scroll,
                                                app.right_pane_height,
                                                app.right_content_lines,
                                            )
                                        };
                                    let max_scroll =
                                        App::max_scroll_offset(content_lines, pane_height);
                                    *scroll = scroll.saturating_add(1).min(max_scroll);
                                } else if is_action(keymap::Action::ScrollUp)
                                    || matches!(key.code, KeyCode::Up)
                                {
                                    if matches!(app.state, AppState::LeftPane) {
                                        app.left_scroll = app.left_scroll.saturating_sub(1);
                                    } else {
                                        app.right_scroll = app.right_scroll.saturating_sub(1);
                                    }
                                } else if is_action(keymap::Action::ScrollPageDown) {
                                    let (scroll, pane_height, content_lines) =
                                        if matches!(app.state, AppState::LeftPane) {
                                            (
                                                &mut app.left_scroll,
                                                app.left_pane_height,
                                                app.left_content_lines,
                                            )
                                        } else {
                                            (
                                                &mut app.right_scroll,
                                                app.right_pane_height,
                                                app.right_content_lines,
                                            )
                                        };
                                    let max_scroll =
                                        App::max_scroll_offset(content_lines, pane_height);
                                    *scroll = scroll.saturating_add(pane_height).min(max_scroll);
                                } else if is_action(keymap::Action::ScrollPageUp) {
                                    if matches!(app.state, AppState::LeftPane) {
                                        app.left_scroll =
                                            app.left_scroll.saturating_sub(app.left_pane_height);
                                    } else {
                                        app.right_scroll =
                                            app.right_scroll.saturating_sub(app.right_pane_height);
                                    }
                                } else if is_action(keymap::Action::ScrollToTop) {
                                    if matches!(app.state, AppState::LeftPane) {
                                        app.left_scroll = 0;
                                    } else {
                                        app.right_scroll = 0;
                                    }
                                } else if is_action(keymap::Action::ScrollToBottom) {
                                    if matches!(app.state, AppState::LeftPane) {
                                        app.left_scroll = app.max_left_scroll();
                                    } else {
                                        app.right_scroll = app.max_right_scroll();
                                    }
                                } else if is_action(keymap::Action::ToggleQueryBar) {
                                    app.query_bar_visible = !app.query_bar_visible;
                                    if !app.query_bar_visible
                                        && matches!(app.state, AppState::QueryInput)
                                    {
                                        app.state = AppState::LeftPane;
                                    }
                                } else if is_action(keymap::Action::ToggleMenu) {
                                    app.side_menu.visible = !app.side_menu.visible;
                                    if app.side_menu.visible {
                                        app.state = AppState::SideMenu;
                                    }
                                }
                            }
                        }
                    }
                    Event::Paste(text) => {
                        if matches!(app.state, AppState::QueryInput) {
                            let cleaned = strip_sgr_mouse_sequences(&text);
                            for ch in cleaned
                                .chars()
                                .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
                                .filter(|c| *c != '\n' && *c != '\r')
                            {
                                app.query_input.textarea.insert_char(ch);
                            }
                            app.query_input.show_suggestions = false;
                            suggestion_active = false;
                            lsp_completions.clear();
                            cached_pipe_type = None;
                            last_edit_at = Instant::now();
                            debounce_pending = true;
                        }
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollDown => {
                            if scroll_input_suppressed(
                                suppress_scroll_until,
                                drop_scroll_backlog_until,
                            ) {
                                continue;
                            }
                            if let Some(pane) = mouse_scroll_pane(app, mouse.column, mouse.row)
                                && can_scroll_in_direction(app, pane, 1)
                            {
                                let _ = apply_mouse_scroll_delta(app, pane, scroll_boost);
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if scroll_input_suppressed(
                                suppress_scroll_until,
                                drop_scroll_backlog_until,
                            ) {
                                continue;
                            }
                            if let Some(pane) = mouse_scroll_pane(app, mouse.column, mouse.row)
                                && can_scroll_in_direction(app, pane, -1)
                            {
                                let _ = apply_mouse_scroll_delta(app, pane, -scroll_boost);
                            }
                        }
                        MouseEventKind::Down(ratatui::crossterm::event::MouseButton::Left) => {
                            focus_state_from_click(app, mouse.column, mouse.row);

                            if mouse.column == app.left_scrollbar_col
                                && row_in_pane(mouse.row, app.left_pane_top, app.left_pane_height)
                            {
                                app.left_scroll = App::scroll_offset_from_row(
                                    mouse.row,
                                    app.left_pane_top,
                                    app.left_pane_height,
                                    app.left_content_lines,
                                );
                                app.drag_target = Some(DragTarget::LeftScrollbar);
                            } else if mouse.column == app.right_scrollbar_col
                                && row_in_pane(mouse.row, app.right_pane_top, app.right_pane_height)
                            {
                                app.right_scroll = App::scroll_offset_from_row(
                                    mouse.row,
                                    app.right_pane_top,
                                    app.right_pane_height,
                                    app.right_content_lines,
                                );
                                app.drag_target = Some(DragTarget::RightScrollbar);
                            }
                        }
                        MouseEventKind::Drag(ratatui::crossterm::event::MouseButton::Left) => {
                            match app.drag_target {
                                Some(DragTarget::LeftScrollbar) => {
                                    app.left_scroll = App::scroll_offset_from_row(
                                        mouse.row,
                                        app.left_pane_top,
                                        app.left_pane_height,
                                        app.left_content_lines,
                                    );
                                }
                                Some(DragTarget::RightScrollbar) => {
                                    app.right_scroll = App::scroll_offset_from_row(
                                        mouse.row,
                                        app.right_pane_top,
                                        app.right_pane_height,
                                        app.right_content_lines,
                                    );
                                }
                                None => {}
                            }
                        }
                        MouseEventKind::Up(ratatui::crossterm::event::MouseButton::Left) => {
                            app.drag_target = None;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            if !scroll_input_suppressed(suppress_scroll_until, drop_scroll_backlog_until)
                && let Some((pane, delta)) = latest_scroll
                && delta != 0
            {
                let _ = apply_mouse_scroll_delta(app, pane, delta);
            }
        }

        if debounce_pending && last_edit_at.elapsed() >= debounce_duration {
            debounce_pending = false;
            let query = app.query_input.textarea.lines()[0].clone();
            let cursor_col = app.query_input.textarea.cursor().1;
            let query_prefix: String = query.chars().take(cursor_col).collect();
            let effective_query = if query.trim().is_empty() {
                ".".to_string()
            } else {
                query.clone()
            };

            if let Some(ref exec) = app.executor {
                if query_prefix.rfind('|').is_none() {
                    cached_pipe_type = None;
                }
                let eq = effective_query.clone();
                let q = query_prefix.clone();
                let input = exec.json_input.clone();
                compute_handle = Some(tokio::task::spawn_blocking(move || {
                    let main_result = Executor::execute_query(&eq, &input);
                    let type_query = Executor::strip_format_op(&q)
                        .map(|(base, _)| base)
                        .unwrap_or_else(|| q.clone());
                    let pipe_type = type_query.rfind('|').and_then(|p| {
                        let prefix = type_query[..p].trim();
                        if prefix.is_empty() {
                            return None;
                        }
                        Executor::execute(prefix, &input)
                            .ok()
                            .and_then(|mut r| {
                                if r.is_empty() {
                                    None
                                } else {
                                    Some(r.swap_remove(0))
                                }
                            })
                            .map(|v| completions::jq_builtins::jq_type_of(&v).to_string())
                    });
                    (main_result, pipe_type)
                }));
                pending_qp = query_prefix.clone();

                if suggestion_active {
                    app.query_input.suggestions = compute_suggestions(
                        &query_prefix,
                        app.executor.as_ref().map(|e| &e.json_input),
                        &lsp_completions,
                        cached_pipe_type.as_deref(),
                    );
                    app.query_input.suggestion_index = 0;
                    app.query_input.suggestion_scroll = 0;
                    let all_exact = !app.query_input.suggestions.is_empty()
                        && app
                            .query_input
                            .suggestions
                            .iter()
                            .all(|s| s.insert_text == query_prefix);
                    if all_exact {
                        app.query_input.show_suggestions = false;
                        suggestion_active = false;
                        lsp_completions.clear();
                        cached_pipe_type = None;
                    } else {
                        app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
                    }
                }
            } else {
                if query_prefix.rfind('|').is_none() {
                    cached_pipe_type = None;
                }
                if suggestion_active {
                    app.query_input.suggestions = compute_suggestions(
                        &query_prefix,
                        None,
                        &lsp_completions,
                        cached_pipe_type.as_deref(),
                    );
                    app.query_input.suggestion_index = 0;
                    app.query_input.suggestion_scroll = 0;
                    let all_exact = !app.query_input.suggestions.is_empty()
                        && app
                            .query_input
                            .suggestions
                            .iter()
                            .all(|s| s.insert_text == query_prefix);
                    if all_exact {
                        app.query_input.show_suggestions = false;
                        suggestion_active = false;
                        lsp_completions.clear();
                        cached_pipe_type = None;
                    } else {
                        app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
                    }
                }
            }

            #[allow(clippy::collapsible_if)]
            if suggestion_active {
                if let Some(ref lsp) = lsp_provider {
                    let _ = lsp.did_change(&query).await;
                    let _ = lsp.completion(&query).await;
                }
            }
        }

        if let Some((ref msg, start)) = footer_message {
            let timeout = if msg.starts_with("Config") { 5 } else { 2 };
            if start.elapsed() >= Duration::from_secs(timeout) {
                footer_message = None;
            }
        }
        app.footer_message = footer_message.as_ref().map(|(m, _)| m.clone());
    }

    if let Some(mut lsp) = lsp_provider {
        let _ = lsp.shutdown().await;
    }

    Ok(())
}

fn row_in_pane(row: u16, pane_top: u16, pane_height: u16) -> bool {
    row >= pane_top && row < pane_top.saturating_add(pane_height)
}

fn mouse_in_left_pane(app: &App<'_>, column: u16, row: u16) -> bool {
    row_in_pane(row, app.left_pane_top, app.left_pane_height) && column <= app.left_scrollbar_col
}

fn mouse_in_right_pane(app: &App<'_>, column: u16, row: u16) -> bool {
    row_in_pane(row, app.right_pane_top, app.right_pane_height)
        && column > app.left_scrollbar_col
        && column <= app.right_scrollbar_col
}

fn focus_state_from_click(app: &mut App<'_>, column: u16, row: u16) {
    if app.query_bar_visible && row < 3 {
        app.state = AppState::QueryInput;
        return;
    }

    if app.side_menu.visible
        && column < 20
        && row_in_pane(row, app.left_pane_top, app.left_pane_height)
    {
        app.state = AppState::SideMenu;
        return;
    }

    if mouse_in_left_pane(app, column, row) {
        app.state = AppState::LeftPane;
    } else if mouse_in_right_pane(app, column, row) {
        app.state = AppState::RightPane;
    }
}

fn should_ignore_query_input_key(key: &ratatui::crossterm::event::KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(c) if c.is_control())
        || (matches!(key.code, KeyCode::Char(_))
            && key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScrollPane {
    Left,
    Right,
}

fn mouse_scroll_pane(app: &App<'_>, column: u16, row: u16) -> Option<ScrollPane> {
    if mouse_in_left_pane(app, column, row) {
        Some(ScrollPane::Left)
    } else if mouse_in_right_pane(app, column, row) {
        Some(ScrollPane::Right)
    } else {
        None
    }
}

fn mouse_scroll_direction(
    app: &App<'_>,
    mouse: &ratatui::crossterm::event::MouseEvent,
) -> Option<(ScrollPane, i8)> {
    let pane = mouse_scroll_pane(app, mouse.column, mouse.row)?;
    match mouse.kind {
        MouseEventKind::ScrollDown => Some((pane, 1)),
        MouseEventKind::ScrollUp => Some((pane, -1)),
        _ => None,
    }
}

fn can_scroll_in_direction(app: &App<'_>, pane: ScrollPane, dir: i8) -> bool {
    match (pane, dir.signum()) {
        (ScrollPane::Left, 1) => app.left_scroll < app.max_left_scroll(),
        (ScrollPane::Left, -1) => app.left_scroll > 0,
        (ScrollPane::Right, 1) => app.right_scroll < app.max_right_scroll(),
        (ScrollPane::Right, -1) => app.right_scroll > 0,
        _ => false,
    }
}

fn is_scroll_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse)
            if matches!(mouse.kind, MouseEventKind::ScrollDown | MouseEventKind::ScrollUp)
    )
}

fn scroll_input_suppressed(
    suppress_scroll_until: Option<Instant>,
    drop_scroll_backlog_until: Option<Instant>,
) -> bool {
    suppress_scroll_until
        .map(|until| Instant::now() <= until)
        .unwrap_or(false)
        || drop_scroll_backlog_until
            .map(|until| Instant::now() <= until)
            .unwrap_or(false)
}

fn should_drop_boundary_scroll_event(app: &App<'_>, event: &Event) -> bool {
    let Event::Mouse(mouse) = event else {
        return false;
    };

    match mouse.kind {
        MouseEventKind::ScrollDown => mouse_scroll_pane(app, mouse.column, mouse.row)
            .map(|pane| !can_scroll_in_direction(app, pane, 1))
            .unwrap_or(false),
        MouseEventKind::ScrollUp => mouse_scroll_pane(app, mouse.column, mouse.row)
            .map(|pane| !can_scroll_in_direction(app, pane, -1))
            .unwrap_or(false),
        _ => false,
    }
}

fn apply_mouse_scroll_delta(app: &mut App<'_>, pane: ScrollPane, delta: i16) -> bool {
    if delta == 0 {
        return false;
    }

    match pane {
        ScrollPane::Left => {
            let prev = app.left_scroll;
            if delta > 0 {
                app.left_scroll = app
                    .left_scroll
                    .saturating_add(delta as u16)
                    .min(app.max_left_scroll());
            } else {
                app.left_scroll = app.left_scroll.saturating_sub((-delta) as u16);
            }
            app.left_scroll != prev
        }
        ScrollPane::Right => {
            let prev = app.right_scroll;
            if delta > 0 {
                app.right_scroll = app
                    .right_scroll
                    .saturating_add(delta as u16)
                    .min(app.max_right_scroll());
            } else {
                app.right_scroll = app.right_scroll.saturating_sub((-delta) as u16);
            }
            app.right_scroll != prev
        }
    }
}

fn strip_sgr_mouse_sequences(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        if let Some(skip) = sgr_mouse_sequence_len(&bytes[i..]) {
            i += skip;
            continue;
        }

        if let Some(ch) = input[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    out
}

fn sgr_mouse_sequence_len(input: &[u8]) -> Option<usize> {
    let mut idx = 0;
    if input.first().copied() == Some(0x1b) {
        idx += 1;
    }

    if input.get(idx).copied() != Some(b'[') || input.get(idx + 1).copied() != Some(b'<') {
        return None;
    }
    idx += 2;

    fn take_digits(input: &[u8], idx: &mut usize) -> bool {
        let start = *idx;
        while input.get(*idx).copied().is_some_and(|b| b.is_ascii_digit()) {
            *idx += 1;
        }
        *idx > start
    }

    if !take_digits(input, &mut idx) || input.get(idx).copied() != Some(b';') {
        return None;
    }
    idx += 1;

    if !take_digits(input, &mut idx) || input.get(idx).copied() != Some(b';') {
        return None;
    }
    idx += 1;

    if !take_digits(input, &mut idx) {
        return None;
    }

    match input.get(idx).copied() {
        Some(b'm') | Some(b'M') => Some(idx + 1),
        _ => None,
    }
}

fn compute_suggestions(
    query_prefix: &str,
    json_input: Option<&serde_json::Value>,
    lsp_completions: &[completions::CompletionItem],
    pipe_context_type: Option<&str>,
) -> Vec<widgets::query_input::Suggestion> {
    let token = current_token(query_prefix);
    let prefix = lsp_pipe_prefix(query_prefix);

    let json_completions = if let Some(input) = json_input {
        completions::json_context::get_completions(query_prefix, input)
    } else {
        Vec::new()
    };

    let builtin_completions: Vec<completions::CompletionItem> = {
        completions::jq_builtins::get_completions(token, pipe_context_type)
            .into_iter()
            .map(|c| completions::CompletionItem {
                insert_text: format!("{}{}", prefix, c.insert_text),
                ..c
            })
            .collect()
    };

    let lsp_patched: Vec<completions::CompletionItem> =
        build_lsp_suggestions(lsp_completions, token, prefix);

    let mut merged = json_completions;
    for item in builtin_completions {
        if !merged
            .iter()
            .any(|i: &completions::CompletionItem| i.label == item.label)
        {
            merged.push(item);
        }
    }
    for item in lsp_patched {
        if !merged
            .iter()
            .any(|i: &completions::CompletionItem| i.label == item.label)
        {
            merged.push(item);
        }
    }

    merged
        .into_iter()
        .map(|i| widgets::query_input::Suggestion {
            label: i.label,
            insert_text: i.insert_text,
        })
        .collect()
}

fn current_token(query: &str) -> &str {
    if let Some(p) = query.rfind('|') {
        query[p + 1..].trim_start()
    } else {
        query
    }
}

fn lsp_pipe_prefix(query: &str) -> &str {
    if let Some(p) = query.rfind('|') {
        &query[..p + 1]
    } else {
        ""
    }
}

fn build_lsp_suggestions(
    cache: &[completions::CompletionItem],
    token: &str,
    prefix: &str,
) -> Vec<completions::CompletionItem> {
    cache
        .iter()
        .filter(|c| c.label.starts_with(token))
        .map(|c| completions::CompletionItem {
            insert_text: format!("{}{}", prefix, c.insert_text),
            ..c.clone()
        })
        .collect()
}

fn cursor_col_after_accept(suggestion: &str) -> u16 {
    if let Some(p) = suggestion.rfind("(\"") {
        (p + 2) as u16
    } else {
        suggestion.chars().count() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jqpp::app::App;

    #[test]
    fn strips_single_sgr_mouse_sequence() {
        let input = "\u{1b}[<65;211;13M";
        assert_eq!(strip_sgr_mouse_sequences(input), "");
    }

    #[test]
    fn strips_repeated_sgr_mouse_sequences_from_paste() {
        let input = "[<65;211;13M[<65;211;13M[<64;211;13M";
        assert_eq!(strip_sgr_mouse_sequences(input), "");
    }

    #[test]
    fn boundary_scroll_events_are_dropped() {
        let mut app = App::new();
        app.left_content_lines = 50;
        app.left_pane_height = 10;
        app.left_pane_top = 4;
        app.left_scrollbar_col = 40;
        app.left_scroll = app.max_left_scroll();

        let evt = Event::Mouse(ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 5,
            row: 8,
            modifiers: KeyModifiers::empty(),
        });

        assert!(should_drop_boundary_scroll_event(&app, &evt));
    }
}
