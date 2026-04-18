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
use clap::{ArgGroup, Parser};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Result,
    Query,
    Input,
}

#[derive(Parser, Debug)]
#[command(version)]
#[command(group(
    ArgGroup::new("output")
        .args(["print_output", "print_query", "print_input"])
        .multiple(false)
))]
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

    /// Print current output to stdout on exit
    #[arg(long, group = "output")]
    print_output: bool,

    /// Print current query to stdout on exit
    #[arg(long, group = "output")]
    print_query: bool,

    /// Print raw input JSON to stdout on exit
    #[arg(long, group = "output")]
    print_input: bool,
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

fn output_mode_from_args(args: &Args) -> Option<OutputMode> {
    if args.print_output {
        Some(OutputMode::Result)
    } else if args.print_query {
        Some(OutputMode::Query)
    } else if args.print_input {
        Some(OutputMode::Input)
    } else {
        None
    }
}

fn selected_output(app: &App<'_>, mode: OutputMode) -> Option<String> {
    match mode {
        OutputMode::Result => {
            if app.results.is_empty() {
                None
            } else {
                Some(Executor::format_results(&app.results, app.raw_output))
            }
        }
        OutputMode::Query => Some(
            app.query_input
                .textarea
                .lines()
                .first()
                .cloned()
                .unwrap_or_default(),
        ),
        OutputMode::Input => Some(
            app.executor
                .as_ref()
                .map(|e| String::from_utf8_lossy(&e.raw_input).into_owned())
                .unwrap_or_default(),
        ),
    }
}

fn parse_input_as_json_or_string(input_data: &[u8]) -> Result<serde_json::Value> {
    if let Ok(json) = serde_json::from_slice(input_data) {
        return Ok(json);
    }

    let text = String::from_utf8(input_data.to_vec())
        .context("Failed to parse input as JSON or UTF-8 text")?;
    let trimmed = text.trim_end_matches(['\n', '\r']);
    if trimmed.chars().any(|c| c.is_whitespace()) {
        return Err(anyhow::anyhow!("Failed to parse input as JSON"));
    }
    Ok(serde_json::Value::String(trimmed.to_string()))
}

fn actual_main(args: Args) -> Result<()> {
    setup_panic_hook(args.debug);
    let output_mode = output_mode_from_args(&args);

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
        let json_input = parse_input_as_json_or_string(&input_data)?;
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

    let app = rt.block_on(run(
        executor,
        args,
        tty_handle,
        keymap,
        config_error,
        output_mode,
    ))?;

    if let Some(mode) = output_mode
        && let Some(text) = selected_output(&app, mode)
    {
        println!("{}", text);
    }

    Ok(())
}

async fn run(
    executor: Option<Executor>,
    args: Args,
    tty_handle: Option<std::fs::File>,
    keymap: keymap::Keymap,
    config_error: Option<String>,
    output_mode: Option<OutputMode>,
) -> Result<App<'static>> {
    // Headless mode: used by integration tests. Start LSP if requested but
    // never touch the terminal — no raw mode, no alternate screen.
    let use_lsp = !args.no_lsp && lsp_on_path();

    let mut app = App::new();
    app.lsp_enabled = use_lsp;
    app.executor = executor;

    if std::env::var("JQPP_SKIP_TTY_CHECK").is_ok() {
        if output_mode.is_some() {
            if let Some(ref exec) = app.executor
                && let Ok(results) = Executor::execute(".", &exec.json_input)
            {
                app.results = results;
                app.error = None;
                app.raw_output = false;
            }
        } else if use_lsp {
            let (lsp_tx, _lsp_rx) = mpsc::channel::<LspMessage>(100);
            let mut provider = LspProvider::new();
            let _ = provider.start(lsp_tx).await;
            tokio::time::sleep(Duration::from_secs(60)).await;
            let _ = provider.shutdown().await;
        }
        return Ok(app);
    }

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
            main_loop(&mut terminal, &mut app, lsp_provider, &mut lsp_rx, &keymap).await?;
            Ok(app)
        }
        None => {
            let backend = CrosstermBackend::new(io::stdout());
            let mut terminal = Terminal::new(backend)?;
            main_loop(&mut terminal, &mut app, lsp_provider, &mut lsp_rx, &keymap).await?;
            Ok(app)
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
                    app.structural_hint_active = false;
                } else {
                    let query_prefix = current_query_prefix(app);
                    if !maybe_activate_structural_hint(app, &query_prefix) {
                        app.structural_hint_active = false;
                        app.query_input.show_suggestions = false;
                        app.query_input.suggestions.clear();
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
                                AppState::RightPane => Some(right_pane_copy_text(app)),
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
                                        let cur = app.query_input.textarea.cursor().1;
                                        let full = app.query_input.textarea.lines()[0].clone();
                                        if let Some((new_text, col)) =
                                            commit_current_string_param_input(&full, cur)
                                        {
                                            app.query_input.textarea =
                                                tui_textarea::TextArea::from(vec![new_text]);
                                            app.query_input.textarea.set_block(
                                                ratatui::widgets::Block::default()
                                                    .title(" Query ")
                                                    .borders(ratatui::widgets::Borders::ALL),
                                            );
                                            app.query_input.textarea.set_cursor_line_style(
                                                ratatui::style::Style::default(),
                                            );
                                            app.query_input.textarea.move_cursor(
                                                tui_textarea::CursorMove::Jump(0, col),
                                            );
                                            app.query_input.show_suggestions = false;
                                            suggestion_active = false;
                                            lsp_completions.clear();
                                            cached_pipe_type = None;
                                            last_edit_at = Instant::now() - debounce_duration;
                                            debounce_pending = true;
                                            continue;
                                        }

                                        let selected = app.query_input.suggestions
                                            [app.query_input.suggestion_index]
                                            .clone();
                                        let suggestion = selected.insert_text;
                                        let (new_text, col) = apply_selected_suggestion(
                                            &suggestion,
                                            selected.detail.as_deref(),
                                            &full,
                                            cur,
                                        );
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
                                        suggestion_active =
                                            starts_context_aware_function_call(&suggestion);
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
                                    if is_action(keymap::Action::AcceptSuggestion)
                                        && app.structural_hint_active
                                        && !app.query_input.suggestions.is_empty()
                                    {
                                        let suggestion = app.query_input.suggestions[0].clone();
                                        let cur = app.query_input.textarea.cursor().1;
                                        let full = app.query_input.textarea.lines()[0].clone();
                                        let suffix: String = full.chars().skip(cur).collect();
                                        let new_text =
                                            format!("{}{}", suggestion.insert_text, suffix);
                                        let col = cursor_col_after_accept(&suggestion.insert_text);
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

                                        app.structural_hint_active = false;
                                        app.query_input.show_suggestions = false;

                                        if suggestion.label == "." {
                                            let query_prefix = current_query_prefix(app);
                                            app.query_input.suggestions = compute_suggestions(
                                                &query_prefix,
                                                app.executor.as_ref().map(|e| &e.json_input),
                                                &lsp_completions,
                                                cached_pipe_type.as_deref(),
                                            );
                                            app.query_input.suggestion_index = 0;
                                            app.query_input.suggestion_scroll = 0;
                                            app.query_input.show_suggestions =
                                                !app.query_input.suggestions.is_empty();
                                            suggestion_active = app.query_input.show_suggestions;
                                        } else {
                                            suggestion_active = false;
                                        }

                                        let new_query = app.query_input.textarea.lines()[0].clone();
                                        clear_dismissed_hint_if_query_changed(app, &new_query);
                                        last_edit_at = Instant::now() - debounce_duration;
                                        debounce_pending = true;
                                    } else if app.query_input.show_suggestions
                                        && !app.query_input.suggestions.is_empty()
                                        && is_action(keymap::Action::AcceptSuggestion)
                                    {
                                        let cur = app.query_input.textarea.cursor().1;
                                        let full = app.query_input.textarea.lines()[0].clone();
                                        if let Some((new_text, col)) =
                                            expand_string_param_prefix_with_tab(
                                                &full,
                                                cur,
                                                &app.query_input.suggestions,
                                                app.query_input.suggestion_index,
                                            )
                                        {
                                            app.query_input.textarea =
                                                tui_textarea::TextArea::from(vec![new_text]);
                                            app.query_input.textarea.set_block(
                                                ratatui::widgets::Block::default()
                                                    .title(" Query ")
                                                    .borders(ratatui::widgets::Borders::ALL),
                                            );
                                            app.query_input.textarea.set_cursor_line_style(
                                                ratatui::style::Style::default(),
                                            );
                                            app.query_input.textarea.move_cursor(
                                                tui_textarea::CursorMove::Jump(0, col),
                                            );
                                            app.query_input.show_suggestions = true;
                                            suggestion_active = true;
                                            last_edit_at = Instant::now() - debounce_duration;
                                            debounce_pending = true;
                                            continue;
                                        }

                                        let selected = app.query_input.suggestions
                                            [app.query_input.suggestion_index]
                                            .clone();
                                        let suggestion = selected.insert_text;
                                        let (new_text, col) = apply_selected_suggestion(
                                            &suggestion,
                                            selected.detail.as_deref(),
                                            &full,
                                            cur,
                                        );
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
                                        suggestion_active =
                                            starts_context_aware_function_call(&suggestion);
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
                                    if app.structural_hint_active
                                        && is_action(keymap::Action::SuggestionUp)
                                    {
                                        let query_prefix = current_query_prefix(app);
                                        open_suggestions_from_structural_hint(
                                            app,
                                            &query_prefix,
                                            &lsp_completions,
                                            cached_pipe_type.as_deref(),
                                            &mut suggestion_active,
                                            true,
                                        );
                                        continue;
                                    }
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
                                    if app.structural_hint_active
                                        && is_action(keymap::Action::SuggestionDown)
                                    {
                                        let query_prefix = current_query_prefix(app);
                                        open_suggestions_from_structural_hint(
                                            app,
                                            &query_prefix,
                                            &lsp_completions,
                                            cached_pipe_type.as_deref(),
                                            &mut suggestion_active,
                                            false,
                                        );
                                        continue;
                                    }
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
                                        app.structural_hint_active = false;
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
                                    let query_prefix = current_query_prefix(app);
                                    if app.structural_hint_active {
                                        dismiss_structural_hint(app, &query_prefix);
                                        suggestion_active = false;
                                        last_esc_at = Some(Instant::now());
                                    } else if app.query_input.show_suggestions {
                                        app.query_input.show_suggestions = false;
                                        suggestion_active = false;
                                        app.structural_hint_active = false;
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
                                        app.structural_hint_active = false;
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
                                } else if matches!(
                                    key.code,
                                    KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End
                                ) {
                                    // Cursor movement doesn't change text so textarea.input()
                                    // returns false — the hint-clearing block below is never
                                    // reached.  Dismiss the structural hint explicitly here
                                    // before forwarding the key to the textarea.
                                    // Clear any active hint without setting
                                    // dismissed_hint_query so it can reappear if
                                    // the cursor returns to the triggering position.
                                    app.structural_hint_active = false;
                                    app.query_input.show_suggestions = false;
                                    app.query_input.suggestions.clear();
                                    app.query_input.textarea.input(key);
                                    // Re-evaluate hint for the new cursor position.
                                    if !suggestion_active {
                                        let new_line = app.query_input.textarea.lines()[0].clone();
                                        let new_col = app.query_input.textarea.cursor().1;
                                        let new_prefix: String =
                                            new_line.chars().take(new_col).collect();
                                        maybe_activate_structural_hint(app, &new_prefix);
                                    }
                                } else if !should_ignore_query_input_key(&key)
                                    && app.query_input.textarea.input(key)
                                {
                                    last_edit_at = Instant::now();
                                    debounce_pending = true;
                                    let query_prefix = current_query_prefix(app);
                                    let next_suggestion_active = suggestion_mode_for_query_edit(
                                        key.code,
                                        &query_prefix,
                                        suggestion_active,
                                    );
                                    suggestion_active = next_suggestion_active;
                                    app.structural_hint_active = false;
                                    if !suggestion_active {
                                        app.query_input.show_suggestions = false;
                                        app.query_input.suggestions.clear();
                                    }
                                    let new_query = app.query_input.textarea.lines()[0].clone();
                                    clear_dismissed_hint_if_query_changed(app, &new_query);
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
                            app.structural_hint_active = false;
                            lsp_completions.clear();
                            cached_pipe_type = None;
                            let new_query = app.query_input.textarea.lines()[0].clone();
                            clear_dismissed_hint_if_query_changed(app, &new_query);
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
            let has_non_exact_suggestion = if suggestion_active {
                app.structural_hint_active = false;
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
                    false
                } else {
                    app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
                    has_non_exact_suggestion_for_prefix(&query_prefix, &app.query_input.suggestions)
                }
            } else {
                false
            };
            let hold_output_during_suggestions = suggestion_active
                && has_non_exact_suggestion
                && should_hold_output_during_suggestions(&query_prefix);
            let effective_query = if query.trim().is_empty() {
                ".".to_string()
            } else {
                query.clone()
            };

            if hold_output_during_suggestions {
                compute_handle = None;
            }

            if let Some(ref exec) = app.executor {
                if query_prefix.rfind('|').is_none() {
                    cached_pipe_type = None;
                }
                if !hold_output_during_suggestions {
                    let eq = effective_query.clone();
                    let q = query_prefix.clone();
                    let input = exec.json_input.clone();
                    compute_handle = Some(tokio::task::spawn_blocking(move || {
                        let main_result = Executor::execute_query(&eq, &input);
                        let type_query = Executor::strip_format_op(&q)
                            .map(|(base, _)| base)
                            .unwrap_or_else(|| q.clone());
                        let pipe_type = if let Some(p) = type_query.rfind('|') {
                            let prefix = type_query[..p].trim();
                            if prefix.is_empty() {
                                None
                            } else {
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
                            }
                        } else {
                            // No pipe — infer type from the query result itself so builtin
                            // suggestions are filtered to what's actually applicable.
                            main_result
                                .as_ref()
                                .ok()
                                .and_then(|(results, _)| results.first())
                                .map(|v| completions::jq_builtins::jq_type_of(v).to_string())
                        };
                        (main_result, pipe_type)
                    }));
                    pending_qp = query_prefix.clone();
                }
            } else if query_prefix.rfind('|').is_none() {
                cached_pipe_type = None;
            }

            if let Some(ref lsp) = lsp_provider {
                let _ = lsp.did_change(&query).await;
                if suggestion_active {
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

fn should_hold_output_during_suggestions(query_prefix: &str) -> bool {
    let token = current_token(query_prefix).trim_end();
    // A bare "." is the identity expression — already complete, don't hold.
    if token == "." {
        return false;
    }
    let Some(last) = token.chars().last() else {
        return false;
    };
    matches!(last, '.' | '|' | '[' | '{' | '(' | ',' | ':')
        || last.is_ascii_alphanumeric()
        || matches!(last, '_' | '-' | '@' | '"' | '\'')
}

fn has_non_exact_suggestion_for_prefix(
    query_prefix: &str,
    suggestions: &[widgets::query_input::Suggestion],
) -> bool {
    suggestions.iter().any(|s| s.insert_text != query_prefix)
}

fn is_field_path_function_call_start(suggestion: &str) -> bool {
    let trimmed = suggestion.trim_end();
    if !(trimmed.ends_with('(') || trimmed.ends_with("()")) {
        return false;
    }
    let Some(open_idx) = trimmed.rfind('(') else {
        return false;
    };
    let before = trimmed[..open_idx].trim_end();
    let fn_name = before
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .rfind(|s| !s.is_empty())
        .unwrap_or("");
    matches!(
        fn_name,
        "sort_by" | "group_by" | "unique_by" | "min_by" | "max_by" | "del" | "path"
    )
}

fn starts_context_aware_function_call(suggestion: &str) -> bool {
    is_field_path_function_call_start(suggestion)
        || completions::json_context::string_param_context(suggestion).is_some()
        || suggestion
            .strip_suffix(')')
            .map(|s| completions::json_context::string_param_context(s).is_some())
            .unwrap_or(false)
}

fn apply_suggestion_with_suffix(suggestion: &str, suffix: &str) -> String {
    let suffix = if suggestion.ends_with(')') && suffix.starts_with(')') {
        &suffix[1..]
    } else {
        suffix
    };
    format!("{}{}", suggestion, suffix)
}

fn is_string_param_value_suggestion(detail: Option<&str>) -> bool {
    detail
        .map(|d| d == "string value" || d == "~string value")
        .unwrap_or(false)
}

fn apply_selected_suggestion(
    insert_text: &str,
    detail: Option<&str>,
    full_query: &str,
    cursor_col: usize,
) -> (String, u16) {
    let query_prefix: String = full_query.chars().take(cursor_col).collect();
    let mut suffix: String = full_query.chars().skip(cursor_col).collect();

    if is_string_param_value_suggestion(detail)
        && completions::json_context::string_param_context(&query_prefix).is_some()
    {
        if let Some(close_idx) = suffix.find(')') {
            suffix = suffix[close_idx + 1..].to_string();
        } else {
            suffix.clear();
        }
        let merged = format!("{}{}", insert_text, suffix);
        return (merged, insert_text.chars().count() as u16);
    }

    let merged = apply_suggestion_with_suffix(insert_text, &suffix);
    let col = if starts_context_aware_function_call(insert_text) {
        cursor_col_after_accept(insert_text)
    } else {
        insert_text.chars().count() as u16
    };
    (merged, col)
}

fn find_unmatched_open_paren(query: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    for (idx, ch) in query.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth < 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn commit_current_string_param_input(full_query: &str, cursor_col: usize) -> Option<(String, u16)> {
    let query_prefix: String = full_query.chars().take(cursor_col).collect();
    let ctx = completions::json_context::string_param_context(&query_prefix)?;
    let open = find_unmatched_open_paren(&query_prefix)?;
    let escaped = ctx.inner_prefix.replace('"', "\\\"");
    let committed = format!("{}\"{}\")", &query_prefix[..open + 1], escaped);
    let suffix: String = full_query.chars().skip(cursor_col).collect();
    let tail = suffix
        .find(')')
        .map(|i| suffix[i + 1..].to_string())
        .unwrap_or_default();
    let new_query = format!("{}{}", committed, tail);
    Some((new_query, committed.chars().count() as u16))
}

fn longest_common_prefix(values: &[String]) -> String {
    let Some(first) = values.first() else {
        return String::new();
    };
    let mut prefix = first.clone();
    for value in values.iter().skip(1) {
        let mut bytes = 0usize;
        for (a, b) in prefix.chars().zip(value.chars()) {
            if a != b {
                break;
            }
            bytes += a.len_utf8();
        }
        prefix.truncate(bytes);
        if prefix.is_empty() {
            break;
        }
    }
    prefix
}

fn is_string_token_delim(ch: char) -> bool {
    matches!(
        ch,
        '\\' | '-' | '_' | '/' | '.' | ' ' | '\t' | ',' | '|' | '@'
    )
}

fn extend_to_next_token_boundary(current: &str, candidate: &str) -> Option<String> {
    if !candidate.starts_with(current) || candidate == current {
        return None;
    }

    let mut out = String::from(current);
    let mut seen_non_delim = false;
    for ch in candidate[current.len()..].chars() {
        if seen_non_delim && is_string_token_delim(ch) {
            break;
        }
        if !is_string_token_delim(ch) {
            seen_non_delim = true;
        }
        out.push(ch);
    }

    if out.len() > current.len() {
        Some(out)
    } else {
        None
    }
}

fn expand_string_param_prefix_with_tab(
    full_query: &str,
    cursor_col: usize,
    suggestions: &[widgets::query_input::Suggestion],
    suggestion_index: usize,
) -> Option<(String, u16)> {
    let query_prefix: String = full_query.chars().take(cursor_col).collect();
    let ctx = completions::json_context::string_param_context(&query_prefix)?;
    let open = find_unmatched_open_paren(&query_prefix)?;

    let candidates: Vec<String> = suggestions
        .iter()
        .filter(|s| is_string_param_value_suggestion(s.detail.as_deref()))
        .map(|s| s.label.clone())
        .collect();
    if candidates.is_empty() {
        return None;
    }

    let preferred = suggestions
        .get(suggestion_index)
        .filter(|s| is_string_param_value_suggestion(s.detail.as_deref()))
        .map(|s| s.label.as_str())
        .or_else(|| {
            candidates
                .iter()
                .find(|c| c.starts_with(ctx.inner_prefix))
                .map(|s| s.as_str())
        })?;

    let extended = if matches!(
        ctx.strategy,
        completions::json_context::StringParamStrategy::Suffix
    ) {
        candidates
            .iter()
            .filter(|c| c.len() > ctx.inner_prefix.len() && c.ends_with(ctx.inner_prefix))
            .min_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
            .cloned()
    } else {
        extend_to_next_token_boundary(ctx.inner_prefix, preferred).or_else(|| {
            let lcp = longest_common_prefix(&candidates);
            if lcp.chars().count() > ctx.inner_prefix.chars().count() {
                Some(lcp)
            } else {
                None
            }
        })
    }?;
    if extended.chars().count() <= ctx.inner_prefix.chars().count() {
        return None;
    }

    let escaped = extended.replace('"', "\\\"");
    let expanded = format!("{}\"{}", &query_prefix[..open + 1], escaped);
    let suffix: String = full_query.chars().skip(cursor_col).collect();
    let new_query = format!("{}{}", expanded, suffix);
    Some((new_query, expanded.chars().count() as u16))
}

fn right_pane_copy_text(app: &App<'_>) -> String {
    if !app.results.is_empty() {
        Executor::format_results(&app.results, app.raw_output)
    } else if let Some(ref err) = app.error {
        err.clone()
    } else {
        Executor::format_results(&app.results, app.raw_output)
    }
}

fn current_query_prefix(app: &App<'_>) -> String {
    let query = app.query_input.textarea.lines()[0].clone();
    let cursor_col = app.query_input.textarea.cursor().1;
    query.chars().take(cursor_col).collect()
}

fn completion_items_to_suggestions(
    items: Vec<completions::CompletionItem>,
) -> Vec<widgets::query_input::Suggestion> {
    items
        .into_iter()
        .map(|i| widgets::query_input::Suggestion {
            label: i.label,
            detail: i.detail,
            insert_text: i.insert_text,
        })
        .collect()
}

fn maybe_activate_structural_hint(app: &mut App<'_>, query_prefix: &str) -> bool {
    if app.dismissed_hint_query.as_deref() == Some(query_prefix) {
        return false;
    }

    let Some(exec) = app.executor.as_ref() else {
        return false;
    };

    if let Some(items) =
        completions::json_context::next_structural_hint(query_prefix, &exec.json_input)
    {
        app.query_input.suggestions = completion_items_to_suggestions(items);
        app.query_input.suggestion_index = 0;
        app.query_input.suggestion_scroll = 0;
        app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
        app.structural_hint_active = app.query_input.show_suggestions;
        return app.structural_hint_active;
    }

    false
}

fn dismiss_structural_hint(app: &mut App<'_>, query_prefix: &str) {
    app.structural_hint_active = false;
    app.query_input.show_suggestions = false;
    app.query_input.suggestions.clear();
    app.dismissed_hint_query = Some(query_prefix.to_string());
}

fn clear_dismissed_hint_if_query_changed(app: &mut App<'_>, query: &str) {
    if app
        .dismissed_hint_query
        .as_deref()
        .is_some_and(|q| q != query)
    {
        app.dismissed_hint_query = None;
    }
}

fn open_suggestions_from_structural_hint(
    app: &mut App<'_>,
    query_prefix: &str,
    lsp_completions: &[completions::CompletionItem],
    cached_pipe_type: Option<&str>,
    suggestion_active: &mut bool,
    select_last: bool,
) {
    let Some(hint) = app.query_input.suggestions.first() else {
        return;
    };

    let trigger = match hint.label.as_str() {
        "[]" => "[",
        "." => ".",
        _ => "",
    };
    if trigger.is_empty() {
        return;
    }

    let prefix = format!("{}{}", query_prefix, trigger);
    app.query_input.suggestions = compute_suggestions(
        &prefix,
        app.executor.as_ref().map(|e| &e.json_input),
        lsp_completions,
        cached_pipe_type,
    );
    app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
    app.structural_hint_active = false;
    *suggestion_active = true;
    if app.query_input.show_suggestions {
        app.query_input.suggestion_index = if select_last {
            app.query_input.suggestions.len().saturating_sub(1)
        } else {
            0
        };
        app.query_input.clamp_scroll();
    }
}

fn is_inside_double_quoted_string(query_prefix: &str) -> bool {
    let mut in_string = false;
    let mut escaped = false;

    for ch in query_prefix.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
        }
    }

    in_string
}

fn suggestion_mode_for_query_edit(
    key_code: KeyCode,
    query_prefix: &str,
    current_active: bool,
) -> bool {
    if is_inside_double_quoted_string(query_prefix)
        && completions::json_context::string_param_context(query_prefix).is_none()
    {
        return false;
    }

    match key_code {
        KeyCode::Char('.')
        | KeyCode::Char('|')
        | KeyCode::Char('{')
        | KeyCode::Char('[')
        | KeyCode::Char(',')
        | KeyCode::Char('@')
        | KeyCode::Backspace
        | KeyCode::Delete => true,
        KeyCode::Char(c) if c.is_alphanumeric() || c == '_' || c == '-' || c == ' ' => {
            current_active
        }
        _ => false,
    }
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
    let in_string_param_context =
        completions::json_context::string_param_context(query_prefix).is_some();
    if is_inside_string_literal(query_prefix) && !in_string_param_context {
        return Vec::new();
    }

    if in_string_param_context {
        let json_only = if let Some(input) = json_input {
            let evaluated =
                evaluated_string_param_input(query_prefix, input).unwrap_or_else(|| input.clone());
            if let Some((head, tail)) = split_string_param_query_prefix(query_prefix) {
                completions::json_context::get_completions(&tail, &evaluated)
                    .into_iter()
                    .map(|i| completions::CompletionItem {
                        insert_text: format!("{}{}", head, i.insert_text),
                        ..i
                    })
                    .collect()
            } else {
                completions::json_context::get_completions(query_prefix, &evaluated)
            }
        } else {
            Vec::new()
        };

        let mut deduped: Vec<completions::CompletionItem> = Vec::new();
        for item in json_only {
            if !deduped
                .iter()
                .any(|d| d.label == item.label && d.insert_text == item.insert_text)
            {
                deduped.push(item);
            }
        }

        return deduped
            .into_iter()
            .map(|i| widgets::query_input::Suggestion {
                label: i.label,
                detail: i.detail,
                insert_text: i.insert_text,
            })
            .collect();
    }

    let token = current_token(query_prefix);
    let fuzzy_token = fuzzy_token_fragment(token);
    let prefix = lsp_pipe_prefix(query_prefix);

    let (eval_input, eval_tail) = if let Some(input) = json_input {
        if let Some((head, tail)) = split_at_last_pipe(query_prefix) {
            let eval_query = Executor::strip_format_op(&head)
                .map(|(base, _)| base)
                .unwrap_or(head);
            let evaluated = Executor::execute(&eval_query, input)
                .ok()
                .and_then(|mut r| {
                    if r.is_empty() {
                        None
                    } else {
                        Some(r.swap_remove(0))
                    }
                })
                .unwrap_or_else(|| input.clone());
            (Some(evaluated), tail)
        } else {
            (Some(input.clone()), query_prefix.to_string())
        }
    } else {
        (None, query_prefix.to_string())
    };

    let json_completions = if let Some(ref input) = eval_input {
        completions::json_context::get_completions(&eval_tail, input)
            .into_iter()
            .map(|i| completions::CompletionItem {
                insert_text: format!("{}{}", prefix, i.insert_text),
                ..i
            })
            .collect()
    } else {
        Vec::new()
    };

    let with_pipe_prefix = |items: Vec<completions::CompletionItem>| {
        items
            .into_iter()
            .map(|c| completions::CompletionItem {
                insert_text: format!("{}{}", prefix, c.insert_text),
                ..c
            })
            .collect::<Vec<_>>()
    };

    let builtin_completions: Vec<completions::CompletionItem> = with_pipe_prefix(
        completions::jq_builtins::get_completions(token, pipe_context_type),
    );

    let all_builtin_completions: Vec<completions::CompletionItem> = with_pipe_prefix(
        completions::jq_builtins::get_completions("", pipe_context_type),
    );

    let fuzzy_builtin_completions: Vec<completions::CompletionItem> =
        if fuzzy_token.is_empty() || !should_offer_builtin_fuzzy(token) {
            Vec::new()
        } else {
            completions::fuzzy::fuzzy_completions(fuzzy_token, &all_builtin_completions)
        };

    let fuzzy_json_completions: Vec<completions::CompletionItem> = if fuzzy_token.is_empty() {
        Vec::new()
    } else if let Some(ref input) = eval_input {
        let fuzzy_tail_prefix = eval_tail.strip_suffix(fuzzy_token).unwrap_or(&eval_tail);
        let all_json_fields = completions::json_context::get_completions(fuzzy_tail_prefix, input);
        completions::fuzzy::fuzzy_completions(fuzzy_token, &all_json_fields)
            .into_iter()
            .map(|i| completions::CompletionItem {
                insert_text: format!("{}{}", prefix, i.insert_text),
                ..i
            })
            .collect()
    } else {
        Vec::new()
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
    for item in fuzzy_json_completions {
        if !merged
            .iter()
            .any(|i: &completions::CompletionItem| i.label == item.label)
        {
            merged.push(item);
        }
    }
    for item in fuzzy_builtin_completions {
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
            detail: i.detail,
            insert_text: i.insert_text,
        })
        .collect()
}

fn active_string_param_prefix_query(query: &str) -> Option<String> {
    completions::json_context::string_param_context(query)?;

    let mut depth: i32 = 0;
    let mut open_paren: Option<usize> = None;
    for (idx, ch) in query.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth < 0 {
                    open_paren = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }
    let open = open_paren?;

    let before_open = query[..open].trim_end();
    if before_open.is_empty() {
        return None;
    }

    let fn_end = before_open.len();
    let mut fn_start = fn_end;
    for (idx, ch) in before_open.char_indices().rev() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            fn_start = idx;
        } else {
            break;
        }
    }
    if fn_start == fn_end {
        return None;
    }

    let prefix = before_open[..fn_start]
        .trim_end()
        .trim_end_matches('|')
        .trim_end();
    if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_string())
    }
}

fn evaluated_string_param_input(
    query_prefix: &str,
    input: &serde_json::Value,
) -> Option<serde_json::Value> {
    let prefix = active_string_param_prefix_query(query_prefix)?;
    let eval_query = Executor::strip_format_op(&prefix)
        .map(|(base, _)| base)
        .unwrap_or(prefix);
    let mut out = Executor::execute(&eval_query, input).ok()?;
    if out.is_empty() {
        Some(serde_json::Value::Null)
    } else if out.len() == 1 {
        Some(out.swap_remove(0))
    } else {
        Some(serde_json::Value::Array(out))
    }
}

fn split_at_last_pipe(query: &str) -> Option<(String, String)> {
    if let Some(p) = query.rfind('|') {
        let head = query[..p].to_string();
        let tail = query[p + 1..].to_string();
        Some((head, tail))
    } else {
        None
    }
}

fn split_string_param_query_prefix(query: &str) -> Option<(String, String)> {
    completions::json_context::string_param_context(query)?;

    let open = find_unmatched_open_paren(query)?;
    let before_open = query[..open].trim_end();
    if before_open.is_empty() {
        return None;
    }

    let fn_end = before_open.len();
    let mut fn_start = fn_end;
    for (idx, ch) in before_open.char_indices().rev() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            fn_start = idx;
        } else {
            break;
        }
    }
    if fn_start == fn_end {
        return None;
    }

    Some((query[..fn_start].to_string(), query[fn_start..].to_string()))
}

fn is_inside_string_literal(query: &str) -> bool {
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in query.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        match quote {
            Some(open) if ch == open => quote = None,
            None if matches!(ch, '"' | '\'') => quote = Some(ch),
            _ => {}
        }
    }

    quote.is_some()
}

fn current_token(query: &str) -> &str {
    if let Some(p) = query.rfind('|') {
        query[p + 1..].trim_start()
    } else {
        query
    }
}

fn fuzzy_token_fragment(token: &str) -> &str {
    token
        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .next()
        .unwrap_or("")
}

fn should_offer_builtin_fuzzy(token: &str) -> bool {
    let t = token.trim_start();
    !t.starts_with('.') && !t.contains('.') && !t.contains('[') && !t.contains('{')
}

fn lsp_pipe_prefix(query: &str) -> &str {
    if let Some(p) = query.rfind('|') {
        &query[..p + 1]
    } else {
        ""
    }
}

fn normalize_lsp_insert_text(insert_text: &str, label: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = insert_text.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '$' {
            if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                i += 2;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                continue;
            }

            if i + 1 < chars.len() && chars[i + 1] == '{' {
                let mut j = i + 2;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j < chars.len() && chars[j] == '}' {
                    i = j + 1;
                    continue;
                }
                if j < chars.len() && chars[j] == ':' {
                    j += 1;
                    while j < chars.len() && chars[j] != '}' {
                        out.push(chars[j]);
                        j += 1;
                    }
                    if j < chars.len() && chars[j] == '}' {
                        i = j + 1;
                        continue;
                    }
                }
            }
        }

        out.push(ch);
        i += 1;
    }

    if out.is_empty() {
        label.to_string()
    } else {
        out
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
            insert_text: format!(
                "{}{}",
                prefix,
                normalize_lsp_insert_text(&c.insert_text, &c.label)
            ),
            ..c.clone()
        })
        .collect()
}

fn cursor_col_after_accept(suggestion: &str) -> u16 {
    if let Some(p) = suggestion.rfind("(\"") {
        (p + 2) as u16
    } else if suggestion.ends_with("()") {
        suggestion.chars().count().saturating_sub(1) as u16
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

    #[test]
    fn holds_output_for_partial_suggestion_token() {
        assert!(should_hold_output_during_suggestions(".items[].na"));
        assert!(should_hold_output_during_suggestions(".items."));
    }

    #[test]
    fn releases_output_for_committed_parent_segment() {
        assert!(!should_hold_output_during_suggestions(".items[]"));
        assert!(!should_hold_output_during_suggestions(".items[] | ."));
    }

    #[test]
    fn right_pane_copy_prefers_results_over_stale_error() {
        let mut app = App::new();
        app.results = vec![serde_json::json!({"name":"alice"})];
        app.error = Some("Error: unexpected EOF".to_string());

        let copied = right_pane_copy_text(&app);

        assert!(copied.contains("alice"));
        assert!(!copied.contains("unexpected EOF"));
    }

    #[test]
    fn right_pane_copy_uses_error_when_no_results() {
        let mut app = App::new();
        app.error = Some("Error: unexpected EOF".to_string());

        let copied = right_pane_copy_text(&app);

        assert_eq!(copied, "Error: unexpected EOF");
    }

    #[test]
    fn ui_validation_holds_output_while_backspacing_partial_token() {
        let suggestions = vec![widgets::query_input::Suggestion {
            label: ".metadata.exported_at".to_string(),
            detail: Some("field".to_string()),
            insert_text: ".metadata.exported_at".to_string(),
        }];

        assert!(has_non_exact_suggestion_for_prefix(
            ".metadata.exported_a",
            &suggestions
        ));
        assert!(should_hold_output_during_suggestions(
            ".metadata.exported_a"
        ));
    }

    #[test]
    fn ui_validation_releases_output_when_query_matches_suggestion() {
        let suggestions = vec![widgets::query_input::Suggestion {
            label: ".metadata.exported_at".to_string(),
            detail: Some("field".to_string()),
            insert_text: ".metadata.exported_at".to_string(),
        }];

        assert!(!has_non_exact_suggestion_for_prefix(
            ".metadata.exported_at",
            &suggestions
        ));
    }

    #[test]
    fn ui_validation_does_not_hold_when_no_suggestions() {
        let suggestions: Vec<widgets::query_input::Suggestion> = Vec::new();
        assert!(!has_non_exact_suggestion_for_prefix(
            ".metadata",
            &suggestions
        ));
    }

    #[test]
    fn fuzzy_results_appear_with_tilde_detail_when_no_exact_prefix() {
        let input = serde_json::json!({"customer_name": "alice"});
        let suggestions = compute_suggestions("up", Some(&input), &[], None);

        assert!(suggestions.iter().any(|s| {
            s.label == "ascii_upcase" && s.detail.as_deref().is_some_and(|d| d.starts_with('~'))
        }));
    }

    #[test]
    fn exact_results_appear_before_fuzzy_results() {
        let input = serde_json::json!({});
        let suggestions = compute_suggestions("st", Some(&input), &[], None);

        let exact_pos = suggestions.iter().position(|s| {
            s.label == "startswith" && !s.detail.as_deref().unwrap_or("").starts_with('~')
        });
        let fuzzy_pos = suggestions.iter().position(|s| {
            s.label == "tostring" && s.detail.as_deref().is_some_and(|d| d.starts_with('~'))
        });

        assert!(exact_pos.is_some(), "expected exact prefix builtin");
        assert!(fuzzy_pos.is_some(), "expected fuzzy builtin");
        assert!(exact_pos.unwrap() < fuzzy_pos.unwrap());
    }

    #[test]
    fn empty_token_produces_no_fuzzy_candidates() {
        let input = serde_json::json!({"customer_name": "alice"});
        let suggestions = compute_suggestions(".customer | ", Some(&input), &[], None);

        assert!(
            suggestions
                .iter()
                .all(|s| !s.detail.as_deref().unwrap_or("").starts_with('~'))
        );
    }

    #[test]
    fn fuzzy_json_field_matches_when_query_starts_with_dot() {
        let input = serde_json::json!({
            "store_region": "EU-NORTH",
            "store_name": "Nordic Widgets"
        });

        let suggestions = compute_suggestions(".egion", Some(&input), &[], None);

        assert!(suggestions.iter().any(|s| {
            s.label == "store_region" && s.detail.as_deref().is_some_and(|d| d.starts_with('~'))
        }));
    }

    #[test]
    fn fuzzy_json_respects_nested_context_path() {
        let input = serde_json::json!({
            "store_name": "Nordic Widgets",
            "orders": [{
                "customer": {
                    "name": "Alice",
                    "email": "alice@example.com"
                }
            }]
        });

        let suggestions = compute_suggestions(".orders[].customer.ame", Some(&input), &[], None);

        assert!(suggestions.iter().any(|s| {
            s.label == "name"
                && s.insert_text == ".orders[].customer.name"
                && s.detail.as_deref().is_some_and(|d| d.starts_with('~'))
        }));
        assert!(!suggestions.iter().any(|s| s.insert_text == ".store_name"));
    }

    #[test]
    fn fuzzy_does_not_offer_builtin_functions_in_dot_path_context() {
        let input = serde_json::json!({
            "orders": [{
                "customer": { "name": "Alice" }
            }]
        });

        let suggestions = compute_suggestions(".orders[].customer.ame", Some(&input), &[], None);

        assert!(!suggestions.iter().any(|s| s.label == "ascii_upcase"));
        assert!(!suggestions.iter().any(|s| s.insert_text == "ascii_upcase"));
    }

    #[test]
    fn suggestions_are_suppressed_inside_function_string_arguments() {
        let input = serde_json::json!("alice");

        let suggestions = compute_suggestions("startswith(\"b", Some(&input), &[], Some("string"));

        assert!(
            suggestions.is_empty(),
            "no completions should appear while editing inside quoted function arguments"
        );
    }

    #[test]
    fn string_literal_detection_handles_escaped_quotes() {
        assert!(is_inside_string_literal("startswith(\"a\\\"b"));
        assert!(!is_inside_string_literal("startswith(\"a\\\"b\")"));
    }

    #[test]
    fn parse_input_accepts_plain_text_as_json_string() {
        let parsed = parse_input_as_json_or_string(b"kakaka\n").unwrap();
        assert_eq!(parsed, serde_json::json!("kakaka"));
    }

    #[test]
    fn parse_input_keeps_valid_json_behavior() {
        let parsed = parse_input_as_json_or_string(br#"{"name":"alice"}"#).unwrap();
        assert_eq!(parsed, serde_json::json!({"name": "alice"}));
    }

    #[test]
    fn parse_input_rejects_non_json_whitespace_text() {
        let err = parse_input_as_json_or_string(b"this is not json").unwrap_err();
        assert!(err.to_string().contains("Failed to parse input as JSON"));
    }

    #[test]
    fn structural_hint_suppressed_when_dismissed_query_matches() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: br#"{"items":[1,2,3]}"#.to_vec(),
            json_input: serde_json::json!({"items": [1, 2, 3]}),
            source_label: "test".to_string(),
        });
        app.dismissed_hint_query = Some(".items".to_string());

        let activated = maybe_activate_structural_hint(&mut app, ".items");

        assert!(!activated);
        assert!(!app.structural_hint_active);
        assert!(!app.query_input.show_suggestions);
    }

    #[test]
    fn structural_hint_activates_for_empty_query_with_root_array() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: br#"[{"name":"Alice"}]"#.to_vec(),
            json_input: serde_json::json!([{"name": "Alice"}]),
            source_label: "test".to_string(),
        });

        let activated = maybe_activate_structural_hint(&mut app, "");

        assert!(activated);
        assert!(app.structural_hint_active);
        assert!(app.query_input.show_suggestions);
        assert_eq!(app.query_input.suggestions[0].label, ".");
    }

    #[test]
    fn cursor_movement_dismisses_structural_hint_without_suppressing_reappearance() {
        // Simulate the state after the [] ghost suggestion has appeared.
        let mut app = App::new();
        app.structural_hint_active = true;
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "[]".to_string(),
            detail: None,
            insert_text: ".items[]".to_string(),
        }];

        // Simulate what the cursor-movement handler does: clear without
        // setting dismissed_hint_query (unlike Esc which sets it).
        app.structural_hint_active = false;
        app.query_input.show_suggestions = false;
        app.query_input.suggestions.clear();

        assert!(
            !app.structural_hint_active,
            "hint should be cleared after cursor move"
        );
        assert!(
            !app.query_input.show_suggestions,
            "dropdown should be hidden"
        );
        assert!(
            app.query_input.suggestions.is_empty(),
            "suggestions should be cleared"
        );
        // dismissed_hint_query must NOT be set — hint must be allowed to reappear
        assert!(
            app.dismissed_hint_query.is_none(),
            "cursor movement must not suppress hint reappearance"
        );
    }

    #[test]
    fn esc_dismisses_structural_hint_and_sets_dismissed_query() {
        let mut app = App::new();
        app.structural_hint_active = true;
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "[]".to_string(),
            detail: None,
            insert_text: ".items[]".to_string(),
        }];

        dismiss_structural_hint(&mut app, ".items");

        assert!(!app.structural_hint_active);
        assert!(!app.query_input.show_suggestions);
        assert!(app.query_input.suggestions.is_empty());
        assert_eq!(app.dismissed_hint_query.as_deref(), Some(".items"));
    }

    #[test]
    fn hint_reappears_when_cursor_returns_to_triggering_position() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: br#"{"items":[{"id":1}]}"#.to_vec(),
            json_input: serde_json::json!({"items": [{"id": 1}]}),
            source_label: "test".to_string(),
        });

        // Hint is showing for ".items"
        maybe_activate_structural_hint(&mut app, ".items");
        assert!(
            app.structural_hint_active,
            "hint should be active at .items"
        );

        // Cursor moves left — hint clears without setting dismissed_hint_query
        app.structural_hint_active = false;
        app.query_input.show_suggestions = false;
        app.query_input.suggestions.clear();
        assert!(app.dismissed_hint_query.is_none());

        // Cursor moves back to ".items" — hint must reappear
        let reactivated = maybe_activate_structural_hint(&mut app, ".items");
        assert!(
            reactivated,
            "hint should reappear when cursor returns to .items"
        );
        assert!(app.structural_hint_active);
        assert!(app.query_input.show_suggestions);
        assert!(!app.query_input.suggestions.is_empty());
    }

    #[test]
    fn builtins_filtered_to_string_type_when_pipe_context_is_string() {
        let input = serde_json::json!({"name": "Alice"});
        // Use a pipe-terminated prefix so the token is empty and all type-filtered
        // builtins are candidates (token="" matches everything).
        let string_suggestions = compute_suggestions(".name | ", Some(&input), &[], Some("string"));
        let all_suggestions = compute_suggestions(".name | ", Some(&input), &[], None);

        assert!(
            string_suggestions.len() < all_suggestions.len(),
            "string-typed suggestions ({}) should be fewer than unfiltered ({})",
            string_suggestions.len(),
            all_suggestions.len()
        );

        // ascii_upcase is a string-only builtin — must appear in string context
        assert!(
            string_suggestions.iter().any(|s| s.label == "ascii_upcase"),
            "ascii_upcase should be suggested for string context"
        );

        // length applies to any type — must appear in both
        assert!(
            string_suggestions.iter().any(|s| s.label == "length"),
            "length should be suggested for string context"
        );
        assert!(
            all_suggestions.iter().any(|s| s.label == "length"),
            "length should be suggested with no context"
        );

        // keys is object/array only — must NOT appear in string context
        assert!(
            !string_suggestions.iter().any(|s| s.label == "keys"),
            "keys should not be suggested for string context"
        );
    }

    #[test]
    fn builtins_filtered_to_array_type_when_pipe_context_is_array() {
        let input = serde_json::json!({"items": [1, 2, 3]});
        let array_suggestions = compute_suggestions(".items | ", Some(&input), &[], Some("array"));

        // map is array-only — must appear
        assert!(
            array_suggestions.iter().any(|s| s.label.starts_with("map")),
            "map should be suggested for array context"
        );
        // ascii_upcase is string-only — must NOT appear
        assert!(
            !array_suggestions.iter().any(|s| s.label == "ascii_upcase"),
            "ascii_upcase should not be suggested for array context"
        );
    }

    #[test]
    fn structural_hint_resets_suggestion_index_to_zero() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: br#"{"items":[1,2,3]}"#.to_vec(),
            json_input: serde_json::json!({"items": [1, 2, 3]}),
            source_label: "test".to_string(),
        });
        app.query_input.suggestion_index = 3;
        app.query_input.suggestion_scroll = 2;

        let activated = maybe_activate_structural_hint(&mut app, ".items");

        assert!(activated);
        assert_eq!(app.query_input.suggestion_index, 0);
        assert_eq!(app.query_input.suggestion_scroll, 0);
    }

    #[test]
    fn cursor_position_enters_parentheses_for_field_path_functions() {
        assert_eq!(cursor_col_after_accept("sort_by()"), 8);
        assert_eq!(cursor_col_after_accept(".orders | sort_by()"), 18);
    }

    #[test]
    fn cursor_position_enters_parentheses_for_string_param_functions() {
        assert_eq!(cursor_col_after_accept("split()"), 6);
        assert_eq!(cursor_col_after_accept("startswith()"), 11);
    }

    #[test]
    fn field_path_function_start_detection_supports_empty_parens() {
        assert!(is_field_path_function_call_start("sort_by()"));
        assert!(is_field_path_function_call_start(".orders | del()"));
        assert!(!is_field_path_function_call_start("map(.)"));
    }

    #[test]
    fn context_aware_function_start_detection_includes_string_param_functions() {
        assert!(starts_context_aware_function_call("sort_by()"));
        assert!(starts_context_aware_function_call("split()"));
        assert!(starts_context_aware_function_call("startswith()"));
        assert!(!starts_context_aware_function_call("map(.)"));
    }

    #[test]
    fn suggestion_accept_drops_redundant_closing_paren_from_suffix() {
        assert_eq!(
            apply_suggestion_with_suffix("split(\"-\")", ")"),
            "split(\"-\")"
        );
        assert_eq!(
            apply_suggestion_with_suffix("split(\"-\")", ") | ."),
            "split(\"-\") | ."
        );
    }

    #[test]
    fn apply_selected_suggestion_for_string_param_replaces_existing_arg_and_moves_to_end() {
        let full = ".[].name|startswith(\"Alice\")";
        let cursor = ".[].name|startswith(\"".chars().count();
        let (new_query, col) = apply_selected_suggestion(
            ".[].name|startswith(\"Bob\")",
            Some("string value"),
            full,
            cursor,
        );
        assert_eq!(new_query, ".[].name|startswith(\"Bob\")");
        assert_eq!(col as usize, ".[].name|startswith(\"Bob\")".chars().count());
    }

    #[test]
    fn apply_selected_suggestion_keeps_function_cursor_inside_parens() {
        let full = "startswith";
        let cursor = full.chars().count();
        let (new_query, col) = apply_selected_suggestion("startswith()", None, full, cursor);
        assert_eq!(new_query, "startswith()");
        assert_eq!(col, 11);
    }

    #[test]
    fn detects_when_cursor_is_inside_double_quoted_string() {
        assert!(is_inside_double_quoted_string(
            ".orders[].customer.customer_id|ascii_downcase|startswith(\"a"
        ));
        assert!(is_inside_double_quoted_string(
            ".foo|startswith(\"escaped \\\" quote"
        ));
    }

    #[test]
    fn detects_when_cursor_is_outside_double_quoted_string() {
        assert!(!is_inside_double_quoted_string(
            ".orders[].customer.customer_id|ascii_downcase|startswith(\"a\")"
        ));
        assert!(!is_inside_double_quoted_string(
            ".orders[].customer.customer_id|ascii_downcase|startswith(\"\")|."
        ));
    }

    #[test]
    fn string_param_quoted_text_edit_keeps_suggestions_active() {
        let q1 = ".orders[].customer.customer_id|ascii_downcase|startswith(\"a";
        let s1 = suggestion_mode_for_query_edit(KeyCode::Char('a'), q1, true);
        assert!(s1);

        let q2 = ".orders[].customer.customer_id|ascii_downcase|startswith(\"";
        let s2 = suggestion_mode_for_query_edit(KeyCode::Backspace, q2, s1);
        assert!(s2);

        let q3 = ".orders[].customer.customer_id|ascii_downcase|startswith(\"b";
        let s3 = suggestion_mode_for_query_edit(KeyCode::Char('b'), q3, s2);
        assert!(s3);
    }

    #[test]
    fn suggestions_for_complex_pipe_chain_in_obj_constructor() {
        let input = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string("examples/string-functions-kitchen-sink.json").unwrap(),
        )
        .unwrap();

        let suggestions = compute_suggestions(
            ".users | sort_by([.role, .email])[] | {",
            Some(&input),
            &[],
            None,
        );

        assert!(
            suggestions.iter().any(|s| s.label == "role"),
            "Expected 'role' in suggestions, but got: {:?}",
            suggestions
        );
        assert!(
            suggestions.iter().any(|s| s.label == "email"),
            "Expected 'email' in suggestions"
        );
    }

    #[test]
    fn suggestions_for_nested_array_field_access() {
        let input = serde_json::json!({
            "orders": [{"customer": {"id": 1, "name": "Alice"}}]
        });

        let suggestions = compute_suggestions(".orders[] | .customer | {", Some(&input), &[], None);

        assert!(
            suggestions.iter().any(|s| s.label == "name"),
            "Expected 'name' in suggestions, but got: {:?}",
            suggestions
        );
    }

    #[test]
    fn compute_suggestions_in_string_param_context_prefers_runtime_candidates() {
        let input = serde_json::json!(["a-b", "c-d"]);
        let s = compute_suggestions("split(\"", Some(&input), &[], Some("string"));
        assert!(s.iter().any(|i| i.label == "-"));
        assert!(!s.iter().any(|i| i.label == "split"));
    }

    #[test]
    fn active_string_param_prefix_query_extracts_pipe_prefix() {
        assert_eq!(
            active_string_param_prefix_query("ascii_upcase|endswith(\""),
            Some("ascii_upcase".to_string())
        );
        assert_eq!(
            active_string_param_prefix_query(".name | ascii_upcase | endswith(\"a"),
            Some(".name | ascii_upcase".to_string())
        );
        assert_eq!(active_string_param_prefix_query("startswith(\""), None);
    }

    #[test]
    fn split_string_param_query_prefix_splits_head_and_tail() {
        assert_eq!(
            split_string_param_query_prefix(".users[].name|endswith("),
            Some((".users[].name|".to_string(), "endswith(".to_string()))
        );
        assert_eq!(
            split_string_param_query_prefix("endswith(\"a"),
            Some(("".to_string(), "endswith(\"a".to_string()))
        );
    }

    #[test]
    fn string_param_suggestions_use_evaluated_pipe_output_value() {
        let input = serde_json::json!("kakaka");
        let s = compute_suggestions(
            "ascii_upcase|endswith(\"",
            Some(&input),
            &[],
            Some("string"),
        );
        assert!(s.iter().any(|i| i.label == "KAKAKA"));
    }

    #[test]
    fn string_param_suggestions_follow_type_changes_through_pipe_chain() {
        let input = serde_json::json!({"n": 12, "s": "hello"});

        let tostring = compute_suggestions(
            ".n | tostring | startswith(\"",
            Some(&input),
            &[],
            Some("string"),
        );
        assert!(tostring.iter().any(|i| i.label == "12"));

        let non_string =
            compute_suggestions(".s | length | startswith(\"", Some(&input), &[], None);
        assert!(non_string.is_empty());
    }

    #[test]
    fn endswith_suggestions_work_after_pipe_expression_prefix() {
        let input = serde_json::json!({
            "users": [{"name": "Alice"}, {"name": "Bob"}, {"name": "Alicia"}]
        });
        let suggestions =
            compute_suggestions(".users[].name|endswith(", Some(&input), &[], Some("string"));

        assert!(suggestions.iter().any(|s| s.label == "Alice"));
        assert!(
            suggestions
                .iter()
                .any(|s| s.insert_text.starts_with(".users[].name|endswith(\""))
        );
    }

    #[test]
    fn enter_commits_current_string_param_prefix_and_closes_call() {
        let full = ".[].name|startswith(\"Ali";
        let cursor = full.chars().count();
        let (new_query, col) = commit_current_string_param_input(full, cursor).unwrap();
        assert_eq!(new_query, ".[].name|startswith(\"Ali\")");
        assert_eq!(col as usize, ".[].name|startswith(\"Ali\")".chars().count());
    }

    #[test]
    fn enter_commit_replaces_existing_param_and_preserves_tail() {
        let full = ".[].name|startswith(\"Ali\") | .age";
        let cursor = ".[].name|startswith(\"Ali".chars().count();
        let (new_query, col) = commit_current_string_param_input(full, cursor).unwrap();
        assert_eq!(new_query, ".[].name|startswith(\"Ali\") | .age");
        assert_eq!(col as usize, ".[].name|startswith(\"Ali\")".chars().count());
    }

    #[test]
    fn tab_expands_string_param_to_longest_common_prefix() {
        let full = "startswith(\"A";
        let cursor = full.chars().count();
        let suggestions = vec![
            widgets::query_input::Suggestion {
                label: "Alice".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alice\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "Alina".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alina\")".to_string(),
            },
        ];

        let (new_query, col) =
            expand_string_param_prefix_with_tab(full, cursor, &suggestions, 0).unwrap();
        assert_eq!(new_query, "startswith(\"Alice");
        assert_eq!(col as usize, "startswith(\"Alice".chars().count());
    }

    #[test]
    fn tab_prefix_expand_noop_when_no_further_common_prefix() {
        let full = "startswith(\"Ali";
        let cursor = full.chars().count();
        let suggestions = vec![
            widgets::query_input::Suggestion {
                label: "Alice".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alice\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "Alina".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alina\")".to_string(),
            },
        ];

        let (new_query, _) =
            expand_string_param_prefix_with_tab(full, cursor, &suggestions, 0).unwrap();
        assert_eq!(new_query, "startswith(\"Alice");
    }

    #[test]
    fn tab_can_extend_across_multiple_token_boundaries() {
        let s = vec![widgets::query_input::Suggestion {
            label: "Alice Smith".to_string(),
            detail: Some("string value".to_string()),
            insert_text: "startswith(\"Alice Smith\")".to_string(),
        }];

        let q1 = "startswith(\"A";
        let c1 = q1.chars().count();
        let (q2, _) = expand_string_param_prefix_with_tab(q1, c1, &s, 0).unwrap();
        assert_eq!(q2, "startswith(\"Alice");

        let c2 = q2.chars().count();
        let (q3, _) = expand_string_param_prefix_with_tab(&q2, c2, &s, 0).unwrap();
        assert_eq!(q3, "startswith(\"Alice Smith");
    }

    #[test]
    fn tab_extends_suffix_from_short_to_longer_suffix_tokens() {
        let s = vec![
            widgets::query_input::Suggestion {
                label: "com".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "endswith(\"com\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "corp.com".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "endswith(\"corp.com\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "@corp.com".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "endswith(\"@corp.com\")".to_string(),
            },
        ];

        let q1 = "endswith(\"com";
        let c1 = q1.chars().count();
        let (q2, _) = expand_string_param_prefix_with_tab(q1, c1, &s, 0).unwrap();
        assert_eq!(q2, "endswith(\"corp.com");

        let c2 = q2.chars().count();
        let (q3, _) = expand_string_param_prefix_with_tab(&q2, c2, &s, 0).unwrap();
        assert_eq!(q3, "endswith(\"@corp.com");
    }

    #[test]
    fn normalize_lsp_insert_text_removes_tabstop_for_string_functions() {
        assert_eq!(
            normalize_lsp_insert_text("startswith($0)", "startswith"),
            "startswith()"
        );
        assert_eq!(
            normalize_lsp_insert_text("endswith(${0})", "endswith"),
            "endswith()"
        );
    }

    #[test]
    fn build_lsp_suggestions_normalizes_snippets_and_keeps_pipe_prefix() {
        let cache = vec![completions::CompletionItem {
            label: "startswith".to_string(),
            detail: None,
            insert_text: "startswith($0)".to_string(),
        }];

        let s = build_lsp_suggestions(&cache, "st", ".users[].name|");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].insert_text, ".users[].name|startswith()");
        assert!(starts_context_aware_function_call(&s[0].insert_text));
        assert_eq!(cursor_col_after_accept(&s[0].insert_text), 25);
    }

    #[test]
    fn punctuation_still_enables_suggestions_outside_string_literals() {
        let q = ".orders[] | sort_by";
        assert!(suggestion_mode_for_query_edit(KeyCode::Char('('), q, false) == false);
        assert!(suggestion_mode_for_query_edit(KeyCode::Char('.'), q, false));
    }
}
