mod app;
mod completions;
mod executor;
mod ui;
mod widgets;

use anyhow::{Context, Result};
use app::App;
use clap::Parser;
use executor::Executor;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, EnableFocusChange, EnableBracketedPaste, DisableBracketedPaste};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Read, IsTerminal, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use completions::lsp::{LspMessage, LspProvider};

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Positional [file]
    file: Option<PathBuf>,

    /// Enable LSP
    #[arg(long)]
    lsp: bool,

    /// Enable debug mode (shows stack traces)
    #[arg(long)]
    debug: bool,
}

struct TtyWriter(std::fs::File);
impl Write for TtyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.write(buf) }
    fn flush(&mut self) -> io::Result<()> { self.0.flush() }
}

struct TerminalGuard {
    tty_handle: Option<std::fs::File>,
}

impl TerminalGuard {
    fn create(tty: Option<&std::fs::File>) -> Result<Self> {
        // crossterm's enable_raw_mode calls tty_fd() internally, which opens
        // /dev/tty when stdin is not a terminal — handles piped input correctly.
        ratatui::crossterm::terminal::enable_raw_mode()
            .context("Failed to enable raw mode")?;

        let tty_clone = tty.and_then(|t| t.try_clone().ok());

        let setup_result = if let Some(tty_handle) = tty {
            let mut writer = TtyWriter(
                tty_handle.try_clone().context("Failed to clone TTY handle for writer")?,
            );
            execute!(writer, EnterAlternateScreen, EnableMouseCapture, EnableFocusChange, EnableBracketedPaste, Hide)
                .context("Failed to setup TTY terminal state")
        } else {
            execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, EnableFocusChange, EnableBracketedPaste, Hide)
                .context("Failed to initialize terminal state")
        };

        if let Err(e) = setup_result {
            let _ = ratatui::crossterm::terminal::disable_raw_mode();
            return Err(e);
        }

        Ok(Self { tty_handle: tty_clone })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        
        if let Some(ref tty) = self.tty_handle {
            if let Ok(cloned) = tty.try_clone() {
                let mut writer = TtyWriter(cloned);
                let _ = execute!(writer, DisableBracketedPaste, LeaveAlternateScreen, DisableMouseCapture, Show);
                return;
            }
        }

        #[cfg(unix)]
        {
            if let Ok(tty) = std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty") {
                let mut writer = TtyWriter(tty);
                let _ = execute!(writer, DisableBracketedPaste, LeaveAlternateScreen, DisableMouseCapture, Show);
                return;
            }
        }
        let mut stdout = io::stdout();
        let _ = execute!(stdout, DisableBracketedPaste, LeaveAlternateScreen, DisableMouseCapture, Show);
    }
}

/// Signal handler for SIGINT / SIGTERM: restore the terminal then exit immediately.
/// `cfmakeraw` clears ISIG so this is only reachable via `kill -INT/-TERM <pid>`,
/// not from ctrl+c inside the TUI (ctrl+c is captured as a key event).
/// We use libc::_exit which is async-signal-safe.
fn on_exit_signal() {
    let _ = ratatui::crossterm::terminal::disable_raw_mode();
    #[cfg(unix)]
    {
        if let Ok(tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
            let mut w = TtyWriter(tty);
            let _ = ratatui::crossterm::execute!(w, LeaveAlternateScreen, DisableMouseCapture, Show);
        } else {
            let mut out = io::stdout();
            let _ = ratatui::crossterm::execute!(out, LeaveAlternateScreen, DisableMouseCapture, Show);
        }
    }
    unsafe { libc::_exit(0); }
}

fn setup_panic_hook(debug: bool) {
    let original_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        #[cfg(unix)]
        {
            if let Ok(tty) = std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty") {
                let mut writer = TtyWriter(tty);
                let _ = execute!(writer, LeaveAlternateScreen, DisableMouseCapture, Show);
            } else {
                let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture, Show);
            }
        }
        #[cfg(not(unix))]
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture, Show);

        if debug {
            original_panic_hook(panic_info);
        } else {
            eprintln!("jqt panicked. Use --debug for more info.");
        }
    }));
}

// ── Completion helpers ────────────────────────────────────────────────────────

/// After accepting a suggestion whose `insert_text` contains `(…)`, the cursor
/// should land *inside* the parentheses so the user can immediately type the
/// argument, rather than after the closing `)`.
///
/// Returns the byte offset (= column for ASCII) to pass to `CursorMove::Jump`.
/// If the text has no `(`, returns the full length (end-of-line).
fn cursor_col_after_accept(insert_text: &str) -> u16 {
    // Use rfind so that in a multi-pipe query like `.a | split(",") | ltrimstr("")`
    // the cursor lands inside the *last* function's argument, not the first one.
    // Prefer landing after `("` so the user can type the argument directly.
    // Fall back to just after `(` for non-string arguments, then end-of-line.
    if let Some(pos) = insert_text.rfind("(\"") {
        (pos + 2) as u16
    } else if let Some(pos) = insert_text.rfind('(') {
        (pos + 1) as u16
    } else {
        insert_text.len() as u16
    }
}

/// Everything from the start of the query up to and including the last `|` and
/// any trailing whitespace.  Used to reconstruct the full replacement when
/// accepting an LSP function-name completion inside a pipe expression.
///
/// `".config | asc"` → `".config | "`
/// `"abc"`           → `""`
fn lsp_pipe_prefix(query: &str) -> &str {
    if let Some(p) = query.rfind('|') {
        let after = &query[p + 1..];
        let spaces = after.len() - after.trim_start().len();
        &query[..p + 1 + spaces]
    } else {
        ""
    }
}

/// The token currently being completed: text after the last `|`, whitespace-
/// trimmed.  Returns the whole query string when there is no pipe.
fn current_token(query: &str) -> &str {
    query
        .rfind('|')
        .map(|p| query[p + 1..].trim_start())
        .unwrap_or(query)
}

/// Filter stale LSP completions by the current typing prefix (client-side) and
/// patch each `insert_text` with the pipe prefix so that accepting a suggestion
/// replaces the whole query rather than just the function name.
fn build_lsp_suggestions(
    lsp_completions: &[completions::CompletionItem],
    token: &str,
    pipe_prefix: &str,
) -> Vec<completions::CompletionItem> {
    lsp_completions
        .iter()
        .filter(|c| token.is_empty() || c.label.starts_with(token))
        .map(|c| completions::CompletionItem {
            label: c.label.clone(),
            detail: c.detail.clone(),
            insert_text: if pipe_prefix.is_empty() {
                c.insert_text.clone()
            } else {
                format!("{}{}", pipe_prefix, c.insert_text)
            },
        })
        .collect()
}

/// Single source of truth for building the merged suggestion list.
///
/// Called from both the debounce tick and the LSP response handler so that
/// every rebuild uses identical logic (type-aware builtins, LSP cache, json
/// context) regardless of which event triggered the refresh.
fn compute_suggestions(
    query: &str,
    json_input: Option<&serde_json::Value>,
    lsp_completions: &[completions::CompletionItem],
    pipe_type: Option<&str>,
) -> Vec<widgets::query_input::Suggestion> {
    let token  = current_token(query);
    let prefix = lsp_pipe_prefix(query);

    // 1. Field-path completions derived from the actual input JSON.
    let json_completions = json_input
        .map(|input| completions::json_context::get_completions(query, input))
        .unwrap_or_default();

    // 2. Type-aware jq built-in functions — only relevant in a pipe context.
    let builtin_completions: Vec<completions::CompletionItem> = if prefix.is_empty() {
        Vec::new()
    } else {
        completions::jq_builtins::get_completions(token, pipe_type)
            .into_iter()
            .map(|c| completions::CompletionItem {
                insert_text: format!("{}{}", prefix, c.insert_text),
                ..c
            })
            .collect()
    };

    // 3. Stale LSP cache filtered by token + pipe prefix.
    let lsp_patched = build_lsp_suggestions(lsp_completions, token, prefix);

    // Merge: json_context → builtins → lsp (first-seen wins to deduplicate).
    let mut merged = json_completions;
    for item in builtin_completions {
        if !merged.iter().any(|i| i.label == item.label) {
            merged.push(item);
        }
    }
    for item in lsp_patched {
        if !merged.iter().any(|i| i.label == item.label) {
            merged.push(item);
        }
    }

    merged
        .into_iter()
        .map(|i| widgets::query_input::Suggestion { label: i.label, insert_text: i.insert_text })
        .collect()
}

fn main() {
    let args = Args::parse();
    if args.debug {
        unsafe {
            let _ = std::env::set_var("RUST_BACKTRACE", "1");
        }
    }
    
    if let Err(e) = actual_main(args) {
        if std::env::var("RUST_BACKTRACE").is_ok() {
            eprintln!("jqt CRITICAL ERROR: {:?}", e);
        } else {
            eprintln!("jqt CRITICAL ERROR: {}", e);
            eprintln!("\nRun with --debug to see a full stack trace.");
        }
        std::process::exit(1);
    }
}

fn get_tty_handle() -> Option<std::fs::File> {
    #[cfg(unix)]
    {
        // 1. Try /dev/tty
        if let Ok(tty) = std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty") {
            return Some(tty);
        }
        
        // 2. Try ttyname on stdout, stderr, then stdin
        for fd in [libc::STDOUT_FILENO, libc::STDERR_FILENO, libc::STDIN_FILENO] {
            if unsafe { libc::isatty(fd) } != 0 {
                let ptr = unsafe { libc::ttyname(fd) };
                if !ptr.is_null() {
                    let path = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_string_lossy().to_string();
                    if let Ok(tty) = std::fs::OpenOptions::new().read(true).write(true).open(&path) {
                        return Some(tty);
                    }
                }
            }
        }
    }
    None
}

fn actual_main(args: Args) -> Result<()> {
    setup_panic_hook(args.debug);
    
    // 1. Read input data first
    let mut input_data = Vec::new();
    let stdin_is_terminal = io::stdin().is_terminal();
    
    if let Some(ref f_path) = args.file {
        input_data = std::fs::read(f_path).context(format!("Failed to read file: {:?}", f_path))?;
    } else if !stdin_is_terminal {
        io::stdin().read_to_end(&mut input_data).context("Failed to read from stdin pipe")?;
    }

    // 2. OPEN TTY Handle for UI output (used as backend writer when stdout is not a terminal).
    // crossterm's enable_raw_mode / event::poll already open /dev/tty internally when
    // stdin is not a TTY, so no dup2 is needed here.
    let tty_handle = get_tty_handle();

    if !stdin_is_terminal && tty_handle.is_none() && std::env::var("JQT_SKIP_TTY_CHECK").is_err() {
        return Err(anyhow::anyhow!("No TTY found for interactive mode while stdin is redirected."));
    }

    let executor = if !input_data.is_empty() {
        let json_input: serde_json::Value = serde_json::from_slice(&input_data)
            .context("Failed to parse input as JSON")?;
        Some(Executor {
            raw_input: input_data,
            json_input,
            source_label: args.file.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "stdin".to_string()),
        })
    } else {
        None
    };

    // ── Signal handling ──────────────────────────────────────────────────────
    // SIGINT / SIGTERM: clean up terminal and exit immediately.
    // NOTE: cfmakeraw clears ISIG, so inside the TUI ctrl+c is a key event,
    // NOT a signal.  These handlers only fire via `kill -INT/-TERM <pid>`.
    // We do NOT intercept SIGTSTP/SIGCONT — doing so breaks crossterm's
    // internal /dev/tty event-reader thread.
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, || on_exit_signal())?;
        signal_hook::low_level::register(signal_hook::consts::SIGTERM, || on_exit_signal())?;
    }
    // ─────────────────────────────────────────────────────────────────────────

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(run(executor, args, tty_handle))
}

async fn run(executor: Option<Executor>, args: Args, tty_handle: Option<std::fs::File>) -> Result<()> {
    let mut app = App::new();
    app.lsp_enabled = args.lsp;
    app.executor = executor;

    let (lsp_tx, mut lsp_rx) = mpsc::channel::<LspMessage>(100);
    let lsp_provider = if args.lsp {
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

    // 3. Initialize Terminal Guard
    let _guard = TerminalGuard::create(tty_handle.as_ref().and_then(|f| f.try_clone().ok()).as_ref())?;
    
    // Choose backend writer
    match tty_handle {
        Some(tty) => {
            let backend = CrosstermBackend::new(TtyWriter(tty));
            let mut terminal = Terminal::new(backend)?;
            main_loop(&mut terminal, &mut app, lsp_provider, &mut lsp_rx).await
        }
        None => {
            let backend = CrosstermBackend::new(io::stdout());
            let mut terminal = Terminal::new(backend)?;
            main_loop(&mut terminal, &mut app, lsp_provider, &mut lsp_rx).await
        }
    }
}

async fn main_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App<'_>,
    lsp_provider: Option<LspProvider>,
    lsp_rx: &mut mpsc::Receiver<LspMessage>,
) -> Result<()> {
    let debounce_duration = Duration::from_millis(80);
    let mut last_edit_at = Instant::now();
    let mut debounce_pending = false;

    // When JQT_KEY_LOG env var is set, log every raw key event to that path.
    // Usage: JQT_KEY_LOG=/tmp/jqt-keys.log jqt < file.json
    // This lets you diagnose which key codes and modifiers the terminal delivers.
    let mut key_log: Option<std::fs::File> = std::env::var("JQT_KEY_LOG").ok().and_then(|path| {
        std::fs::OpenOptions::new().create(true).append(true).open(path).ok()
    });

    // Run initial query (.) so output pane is populated before user types.
    if let Some(ref exec) = app.executor {
        let input = exec.json_input.clone();
        if let Ok(Ok(results)) = tokio::task::spawn_blocking(move || {
            executor::Executor::execute(".", &input)
        }).await {
            app.results = results;
        }
    }

    let mut footer_message: Option<(String, Instant)> = None;
    let mut lsp_completions: Vec<completions::CompletionItem> = Vec::new();
    // True while the user is actively in a completion context (typed a trigger
    // character like `.`, `|`, `{` and hasn't yet accepted or dismissed).
    let mut suggestion_active = false;
    // The jq type ("string", "number", "array", …) produced by the expression
    // before the last `|`.  Kept in sync by the debounce block and shared with
    // the LSP response handler so both use identical type-aware filtering.
    let mut cached_pipe_type: Option<String> = None;
    // Track when Esc was last pressed in QueryInput so a second Esc within
    // 500 ms clears the query bar (double-Esc-to-clear).
    let mut last_esc_at: Option<Instant> = None;
    // In-flight background query computation.  We store the JoinHandle rather
    // than awaiting it so the event loop stays fully responsive while jaq runs.
    type ComputeResult = (anyhow::Result<(Vec<serde_json::Value>, bool)>, Option<String>);
    let mut compute_handle: Option<tokio::task::JoinHandle<ComputeResult>> = None;
    // The query prefix (text up to cursor) that launched the in-flight compute.
    let mut pending_qp: String = String::new();

    while app.running {
        // ── Poll in-flight background compute ────────────────────────────────
        // Awaiting a finished JoinHandle is always instant; this never blocks.
        if let Some(ref handle) = compute_handle {
            if handle.is_finished() {
                match compute_handle.take().unwrap().await {
                    Ok((Ok((results, raw)), pipe_type)) => {
                        app.results = results;
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
                // Refresh suggestions now that we have the final pipe type.
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
                        && app.query_input.suggestions.iter().all(|s| s.insert_text == pending_qp);
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

        terminal.draw(|f| ui::draw(f, app)).context("Failed to draw TUI frame")?;

        while let Ok(msg) = lsp_rx.try_recv() {
            match msg {
                LspMessage::Status(s) => {
                    app.lsp_status = if s == "ready" { None } else { Some(s) };
                }
                LspMessage::Diagnostic(d) => {
                    app.lsp_diagnostic = d;
                }
                LspMessage::Completions(c) => {
                    // Only refresh the cache on non-empty responses.  When jq-lsp
                    // returns 0 (e.g. `as` is a keyword), we keep the previous
                    // results so the dropdown stays visible while the user types.
                    if !c.is_empty() {
                        lsp_completions = c;
                    }
                    if suggestion_active {
                        let query_line = app.query_input.textarea.lines()[0].clone();
                        let cur = app.query_input.textarea.cursor().1;
                        let query_prefix: String = query_line.chars().take(cur).collect();
                        // Use the same helper as the debounce block so type-aware
                        // builtins are included here too (prevents the flicker where
                        // the LSP response overwrites the type-filtered list).
                        app.query_input.suggestions = compute_suggestions(
                            &query_prefix,
                            app.executor.as_ref().map(|e| &e.json_input),
                            &lsp_completions,
                            cached_pipe_type.as_deref(),
                        );
                        app.query_input.suggestion_index = 0;
                        app.query_input.suggestion_scroll = 0;
                        let all_exact = !app.query_input.suggestions.is_empty()
                            && app.query_input.suggestions.iter().all(|s| s.insert_text == query_prefix);
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
        }

        if event::poll(Duration::from_millis(20)).context("Failed to poll for terminal events")? {
            match event::read().context("Failed to read terminal event")? {
                Event::FocusGained => {
                    terminal.clear().ok();
                }
                Event::Key(key) => {
                    if let Some(ref mut log) = key_log {
                        let _ = writeln!(&*log, "key: {:?} mods: {:?} kind: {:?}", key.code, key.modifiers, key.kind);
                    }
                    // ── Global: exit ──────────────────────────────────────────
                    // Ctrl+C (0x03) and Ctrl+Q (0x11) both quit.  We check both
                    // upper and lower case variants because some terminals/shells
                    // deliver Ctrl+letter as uppercase.  Ctrl+C is NOT subject to
                    // IXON/XON-XOFF interception (unlike 0x11).
                    // Esc also quits from pane views (not from query input where
                    // it dismisses the suggestion dropdown).
                    let is_ctrl_quit = key.modifiers.contains(KeyModifiers::CONTROL)
                        && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')
                                            | KeyCode::Char('c') | KeyCode::Char('C'));
                    let is_pane_quit = !matches!(app.state, app::AppState::QueryInput)
                        && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc);
                    if is_ctrl_quit || is_pane_quit {
                        app.running = false;
                        continue;
                    }

                    // ── Global: copy focused pane content ────────────────────
                    let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('y'))
                        || (key.modifiers.contains(KeyModifiers::SUPER) && key.code == KeyCode::Char('c'));
                    if is_copy {
                        let text = match app.state {
                            app::AppState::QueryInput => Some(app.query_input.textarea.lines()[0].clone()),
                            app::AppState::LeftPane => app.executor.as_ref().map(|e| String::from_utf8_lossy(&e.raw_input).into_owned()),
                            app::AppState::RightPane => {
                                if let Some(ref err) = app.error {
                                    Some(err.clone())
                                } else {
                                    Some(executor::Executor::format_results(&app.results, app.raw_output))
                                }
                            }
                            app::AppState::SideMenu => None,
                        };
                        if let Some(t) = text {
                            // arboard::Clipboard::new() can block on macOS waiting for
                            // the Objective-C run loop.  Run it on a detached OS thread
                            // so the event loop is never stalled.
                            std::thread::spawn(move || {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    let _ = cb.set_text(t);
                                }
                            });
                            footer_message = Some(("copied".to_string(), Instant::now()));
                        }
                        continue;
                    }

                    match app.state {
                        app::AppState::QueryInput => {
                            if key.code == KeyCode::Enter {
                                if app.query_input.show_suggestions && !app.query_input.suggestions.is_empty() {
                                    // Accept highlighted suggestion, preserving any text after cursor.
                                    let suggestion = app.query_input.suggestions[app.query_input.suggestion_index].insert_text.clone();
                                    let cur = app.query_input.textarea.cursor().1;
                                    let full = app.query_input.textarea.lines()[0].clone();
                                    let suffix: String = full.chars().skip(cur).collect();
                                    let new_text = format!("{}{}", suggestion, suffix);
                                    let col = cursor_col_after_accept(&suggestion);
                                    app.query_input.textarea = tui_textarea::TextArea::from(vec![new_text]);
                                    app.query_input.textarea.set_block(
                                        ratatui::widgets::Block::default()
                                            .title(" Query ")
                                            .borders(ratatui::widgets::Borders::ALL),
                                    );
                                    app.query_input.textarea.set_cursor_line_style(ratatui::style::Style::default());
                                    app.query_input.textarea.move_cursor(tui_textarea::CursorMove::Jump(0, col));
                                    app.query_input.show_suggestions = false;
                                    suggestion_active = false;
                                    lsp_completions.clear(); cached_pipe_type = None;
                                    // Trigger immediate live update
                                    last_edit_at = Instant::now() - debounce_duration;
                                    debounce_pending = true;
                                } else {
                                    // Explicit execute
                                    app.query_input.show_suggestions = false;
                                    suggestion_active = false;
                                    let query = app.query_input.textarea.lines()[0].clone();
                                    app.query_input.push_history(query.clone());
                                    if let Some(ref exec) = app.executor {
                                        match executor::Executor::execute_query(&query, &exec.json_input) {
                                            Ok((results, raw)) => { app.results = results; app.error = None; app.raw_output = raw; }
                                            Err(e) => { app.error = Some(e.to_string()); app.results = Vec::new(); app.raw_output = false; }
                                        }
                                    }
                                }
                            } else if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                                let output = executor::Executor::format_results(&app.results, app.raw_output);
                                if std::fs::write("jqt-output.json", output).is_ok() {
                                    footer_message = Some(("saved".to_string(), Instant::now()));
                                }
                            } else if key.code == KeyCode::Tab {
                                if app.query_input.show_suggestions && !app.query_input.suggestions.is_empty() {
                                    // Accept ghost-text / highlighted suggestion, preserving text after cursor.
                                    let suggestion = app.query_input.suggestions[app.query_input.suggestion_index].insert_text.clone();
                                    let cur = app.query_input.textarea.cursor().1;
                                    let full = app.query_input.textarea.lines()[0].clone();
                                    let suffix: String = full.chars().skip(cur).collect();
                                    let new_text = format!("{}{}", suggestion, suffix);
                                    let col = cursor_col_after_accept(&suggestion);
                                    app.query_input.textarea = tui_textarea::TextArea::from(vec![new_text]);
                                    app.query_input.textarea.set_block(
                                        ratatui::widgets::Block::default()
                                            .title(" Query ")
                                            .borders(ratatui::widgets::Borders::ALL),
                                    );
                                    app.query_input.textarea.set_cursor_line_style(ratatui::style::Style::default());
                                    app.query_input.textarea.move_cursor(tui_textarea::CursorMove::Jump(0, col));
                                    app.query_input.show_suggestions = false;
                                    suggestion_active = false;
                                    lsp_completions.clear(); cached_pipe_type = None;
                                    last_edit_at = Instant::now() - debounce_duration;
                                    debounce_pending = true;
                                } else {
                                    // Navigate to next pane
                                    app.next_pane();
                                }
                            } else if key.code == KeyCode::BackTab {
                                app.query_input.show_suggestions = false;
                                suggestion_active = false;
                                app.prev_pane();
                            } else if key.code == KeyCode::Up {
                                if app.query_input.show_suggestions {
                                    if app.query_input.suggestion_index > 0 {
                                        app.query_input.suggestion_index -= 1;
                                        app.query_input.clamp_scroll();
                                    } else {
                                        app.query_input.show_suggestions = false;
                                        suggestion_active = false;
                                        lsp_completions.clear(); cached_pipe_type = None;
                                    }
                                } else if suggestion_active && !app.query_input.suggestions.is_empty() {
                                    // Re-open the dropdown if we have cached suggestions.
                                    app.query_input.show_suggestions = true;
                                    app.query_input.suggestion_index =
                                        app.query_input.suggestions.len().saturating_sub(1);
                                    app.query_input.clamp_scroll();
                                } else {
                                    // Navigate history when no suggestion context.
                                    app.query_input.history_up();
                                }
                            } else if key.code == KeyCode::Down {
                                if app.query_input.show_suggestions {
                                    if app.query_input.suggestion_index + 1 < app.query_input.suggestions.len() {
                                        app.query_input.suggestion_index += 1;
                                        app.query_input.clamp_scroll();
                                    }
                                } else {
                                    // Down always triggers context-based suggestions; if none are
                                    // cached yet, set suggestion_active and fire an immediate debounce.
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
                                    // First Esc: dismiss suggestions, arm double-Esc timer.
                                    app.query_input.show_suggestions = false;
                                    suggestion_active = false;
                                    lsp_completions.clear(); cached_pipe_type = None;
                                    last_esc_at = Some(Instant::now());
                                } else if last_esc_at
                                    .map(|t| t.elapsed() < Duration::from_millis(500))
                                    .unwrap_or(false)
                                {
                                    // Second Esc within 500 ms: clear the query bar.
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
                                    lsp_completions.clear(); cached_pipe_type = None;
                                    last_esc_at = None;
                                    // Re-run the default "." query so output reflects empty filter.
                                    last_edit_at = Instant::now() - debounce_duration;
                                    debounce_pending = true;
                                } else {
                                    // Esc with no suggestions: arm the timer, wait for second.
                                    last_esc_at = Some(Instant::now());
                                }
                            } else if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('t') {
                                app.query_bar_visible = !app.query_bar_visible;
                                if !app.query_bar_visible {
                                    app.state = app::AppState::LeftPane;
                                }
                            } else if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('m') {
                                app.side_menu.visible = !app.side_menu.visible;
                                if app.side_menu.visible { app.state = app::AppState::SideMenu; }
                                else if matches!(app.state, app::AppState::SideMenu) { app.state = app::AppState::QueryInput; }
                            } else if app.query_input.textarea.input(key) {
                                last_edit_at = Instant::now();
                                debounce_pending = true;
                                // Only trigger suggestions on explicit trigger characters.
                                // Keep active while typing an identifier prefix (filtering).
                                // Space is also kept alive — after `|` the user types a space
                                // before the function name and we need suggestions to persist.
                                // Clear on anything else (backspace, closing brackets, etc.).
                                match key.code {
                                    KeyCode::Char('.') | KeyCode::Char('|')
                                    | KeyCode::Char('{') | KeyCode::Char('[')
                                    | KeyCode::Char(',') | KeyCode::Char('@') => {
                                        // `@` starts a format-string operator (@csv, @base64, …)
                                        suggestion_active = true;
                                    }
                                    KeyCode::Char(c)
                                        if c.is_alphanumeric() || c == '_' || c == '-' || c == ' ' =>
                                    {
                                        // keep suggestion_active as-is — filtering ongoing prefix
                                    }
                                    KeyCode::Backspace | KeyCode::Delete => {
                                        // Erasing always re-arms completion context so suggestions
                                        // continue to appear as the user refines or backs out of
                                        // a completed term (even after an accept/dismiss).
                                        suggestion_active = true;
                                    }
                                    _ => {
                                        suggestion_active = false;
                                        app.query_input.show_suggestions = false;
                                    }
                                }
                            }
                        }
                        app::AppState::SideMenu => {
                            match key.code {
                                KeyCode::Tab => app.next_pane(),
                                KeyCode::BackTab => app.prev_pane(),
                                KeyCode::Up => {
                                    if app.side_menu.selected > 0 { app.side_menu.selected -= 1; }
                                    else { app.side_menu.selected = app.side_menu.items.len() - 1; }
                                }
                                KeyCode::Down => {
                                    if app.side_menu.selected + 1 < app.side_menu.items.len() { app.side_menu.selected += 1; }
                                    else { app.side_menu.selected = 0; }
                                }
                                _ => {}
                            }
                            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('m') {
                                app.side_menu.visible = false;
                                app.state = app::AppState::QueryInput;
                            }
                        }
                        _ => {
                            match key.code {
                                KeyCode::Tab => app.next_pane(),
                                KeyCode::BackTab => app.prev_pane(),
                                KeyCode::Char('j') | KeyCode::Down => {
                                    if matches!(app.state, app::AppState::LeftPane) { app.left_scroll += 1; }
                                    else { app.right_scroll += 1; }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    if matches!(app.state, app::AppState::LeftPane) { app.left_scroll = app.left_scroll.saturating_sub(1); }
                                    else { app.right_scroll = app.right_scroll.saturating_sub(1); }
                                }
                                _ => {}
                            }
                            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('t') {
                                app.query_bar_visible = !app.query_bar_visible;
                                if !app.query_bar_visible && matches!(app.state, app::AppState::QueryInput) {
                                    app.state = app::AppState::LeftPane;
                                }
                            } else if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('m') {
                                app.side_menu.visible = !app.side_menu.visible;
                                if app.side_menu.visible { app.state = app::AppState::SideMenu; }
                            }
                        }
                    }
                }
                Event::Paste(text) => {
                    // Bracketed paste: the whole paste arrives as one event.
                    // Insert it into the query bar (first line only — jq queries
                    // are single-line), suppress suggestions, and schedule a
                    // debounce so the query executes once after the paste settles.
                    if matches!(app.state, app::AppState::QueryInput) {
                        // tui-textarea has no direct "insert string" API, so we
                        // feed it as a series of characters.  This is still much
                        // faster than receiving them as individual key events
                        // because we skip all suggestion / LSP logic here.
                        for ch in text.chars().filter(|c| *c != '\n' && *c != '\r') {
                            app.query_input.textarea.insert_char(ch);
                        }
                        app.query_input.show_suggestions = false;
                        suggestion_active = false;
                        lsp_completions.clear();
                        cached_pipe_type = None;
                        // Use the full debounce delay so the query fires once
                        // when typing stops, not on every pasted character.
                        last_edit_at = Instant::now();
                        debounce_pending = true;
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollDown => {
                            if matches!(app.state, app::AppState::LeftPane) { app.left_scroll += 1; }
                            else if matches!(app.state, app::AppState::RightPane) { app.right_scroll += 1; }
                        }
                        MouseEventKind::ScrollUp => {
                            if matches!(app.state, app::AppState::LeftPane) { app.left_scroll = app.left_scroll.saturating_sub(1); }
                            else if matches!(app.state, app::AppState::RightPane) { app.right_scroll = app.right_scroll.saturating_sub(1); }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if debounce_pending && last_edit_at.elapsed() >= debounce_duration {
            debounce_pending = false;
            let query = app.query_input.textarea.lines()[0].clone();
            // For completions, use only the text up to the cursor so that editing
            // in the middle of a query offers the right suggestions for that position.
            let cursor_col = app.query_input.textarea.cursor().1;
            let query_prefix: String = query.chars().take(cursor_col).collect();
            let effective_query = if query.trim().is_empty() { ".".to_string() } else { query.clone() };

            if let Some(ref exec) = app.executor {
                let eq = effective_query.clone();
                let q = query_prefix.clone();
                let input = exec.json_input.clone();
                // Spawn without awaiting — result is polled at the top of each loop
                // iteration so the event loop (and Ctrl+C) always stay responsive.
                // Drop any in-flight handle; the new query supersedes it.
                compute_handle = Some(tokio::task::spawn_blocking(move || {
                    let main_result = executor::Executor::execute_query(&eq, &input);
                    // Detect pipe-prefix output type for context-aware completions.
                    let type_query = executor::Executor::strip_format_op(&q)
                        .map(|(base, _)| base)
                        .unwrap_or_else(|| q.clone());
                    let pipe_type = type_query.rfind('|').and_then(|p| {
                        let prefix = type_query[..p].trim();
                        if prefix.is_empty() { return None; }
                        executor::Executor::execute(prefix, &input)
                            .ok()
                            .and_then(|mut r| if r.is_empty() { None } else { Some(r.swap_remove(0)) })
                            .map(|v| completions::jq_builtins::jq_type_of(&v).to_string())
                    });
                    (main_result, pipe_type)
                }));
                pending_qp = query_prefix.clone();

                // Show preliminary suggestions immediately (using cached pipe type).
                // They will be refreshed when the compute result arrives.
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
                        && app.query_input.suggestions.iter().all(|s| s.insert_text == query_prefix);
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
                // No input file: update pipe type synchronously (nothing to execute).
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
                        && app.query_input.suggestions.iter().all(|s| s.insert_text == query_prefix);
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

            // Dispatch LSP requests when active so function completions
            // (e.g. after `|`) arrive even when json_context has nothing to offer.
            if suggestion_active {
                if let Some(ref lsp) = lsp_provider {
                    let _ = lsp.did_change(&query).await;
                    let _ = lsp.completion(&query).await;
                }
            }
        }

        if let Some((_, start)) = footer_message {
            if start.elapsed() >= Duration::from_secs(2) { footer_message = None; }
        }
        app.footer_message = footer_message.as_ref().map(|(m, _)| m.clone());
    }

    if let Some(mut lsp) = lsp_provider {
        let _ = lsp.shutdown().await;
    }

    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use completions::CompletionItem;

    fn item(label: &str) -> CompletionItem {
        CompletionItem { label: label.to_string(), detail: None, insert_text: label.to_string() }
    }

    // ── lsp_pipe_prefix ───────────────────────────────────────────────────────

    #[test]
    fn pipe_prefix_with_pipe_and_space() {
        assert_eq!(lsp_pipe_prefix(".config | asc"), ".config | ");
    }

    #[test]
    fn pipe_prefix_with_pipe_no_trailing_space() {
        assert_eq!(lsp_pipe_prefix(".config |asc"), ".config |");
    }

    #[test]
    fn pipe_prefix_no_pipe() {
        assert_eq!(lsp_pipe_prefix(".config"), "");
        assert_eq!(lsp_pipe_prefix(""), "");
    }

    // ── current_token ─────────────────────────────────────────────────────────

    #[test]
    fn token_after_pipe() {
        assert_eq!(current_token(".config | asc"), "asc");
        assert_eq!(current_token(".config | "), "");
    }

    #[test]
    fn token_no_pipe_returns_whole_query() {
        assert_eq!(current_token(".config"), ".config");
        assert_eq!(current_token(""), "");
    }

    // ── build_lsp_suggestions ─────────────────────────────────────────────────

    #[test]
    fn lsp_suggestions_prepend_pipe_prefix() {
        let items = vec![item("ascii_upcase"), item("ascii_downcase"), item("split")];
        let result = build_lsp_suggestions(&items, "asc", ".config | ");
        // Only items whose label starts with "asc"
        assert_eq!(result.len(), 2, "got: {:?}", result.iter().map(|c| &c.label).collect::<Vec<_>>());
        assert!(result.iter().all(|c| c.insert_text.starts_with(".config | ascii_")),
            "insert_text must include pipe prefix: {:?}", result.iter().map(|c| &c.insert_text).collect::<Vec<_>>());
    }

    #[test]
    fn lsp_suggestions_no_pipe_no_prefix() {
        let items = vec![item("ascii_upcase")];
        let result = build_lsp_suggestions(&items, "asc", "");
        assert_eq!(result[0].insert_text, "ascii_upcase");
    }

    #[test]
    fn lsp_suggestions_empty_token_shows_all() {
        let items = vec![item("ascii_upcase"), item("split"), item("test")];
        let result = build_lsp_suggestions(&items, "", "");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn lsp_suggestions_no_match() {
        let items = vec![item("split"), item("test")];
        let result = build_lsp_suggestions(&items, "asc", ".config | ");
        assert!(result.is_empty(), "expected empty, got: {:?}", result);
    }

    // ── stale cache filtered to current token (anti-flicker) ─────────────────

    #[test]
    fn stale_cache_with_as_prefix_still_shows_ascii_completions() {
        // jq-lsp returns 0 for "as" (keyword); we keep old cache of ascii_* fns.
        let cache = vec![item("ascii_upcase"), item("ascii_downcase")];
        // Current token = "as" — both ascii_ labels start with "as"
        let result = build_lsp_suggestions(&cache, "as", ".config | ");
        assert_eq!(result.len(), 2, "stale completions must still appear for 'as' prefix");
        assert!(result.iter().all(|c| c.insert_text.starts_with(".config | ")));
    }

    // ── executor result cap ───────────────────────────────────────────────────

    #[test]
    fn executor_caps_results_at_10k() {
        use serde_json::json;
        // Build a 15K-element array; query `.[]` should return at most 10K.
        let arr: Vec<serde_json::Value> = (0..15_000).map(|i| json!(i)).collect();
        let input = serde_json::Value::Array(arr);
        let results = executor::Executor::execute(".[]", &input).unwrap();
        assert_eq!(results.len(), 10_000, "result cap must be 10 000, got {}", results.len());
    }

    // ── Type detection integration ────────────────────────────────────────────
    // Verify that executing the pipe-prefix expression and calling jq_type_of
    // yields the correct type string for each JSON kind.

    fn detect_pipe_prefix_type(input: &serde_json::Value, prefix_expr: &str) -> String {
        executor::Executor::execute(prefix_expr, input)
            .ok()
            .and_then(|mut r| if r.is_empty() { None } else { Some(r.swap_remove(0)) })
            .map(|v| completions::jq_builtins::jq_type_of(&v).to_string())
            .unwrap_or_default()
    }

    #[test]
    fn pipe_prefix_type_string() {
        use serde_json::json;
        let input = json!({"config": {"name": "hello"}});
        assert_eq!(detect_pipe_prefix_type(&input, ".config.name"), "string");
    }

    #[test]
    fn pipe_prefix_type_number() {
        use serde_json::json;
        let input = json!({"value": 3.14});
        assert_eq!(detect_pipe_prefix_type(&input, ".value"), "number");
    }

    #[test]
    fn pipe_prefix_type_array() {
        use serde_json::json;
        let input = json!({"items": [1, 2, 3]});
        assert_eq!(detect_pipe_prefix_type(&input, ".items"), "array");
    }

    #[test]
    fn pipe_prefix_type_object() {
        use serde_json::json;
        let input = json!({"meta": {"version": 1}});
        assert_eq!(detect_pipe_prefix_type(&input, ".meta"), "object");
    }

    #[test]
    fn pipe_prefix_type_boolean() {
        use serde_json::json;
        let input = json!({"active": true});
        assert_eq!(detect_pipe_prefix_type(&input, ".active"), "boolean");
    }

    // ── Type-aware builtin integration ────────────────────────────────────────
    // Combines type detection with jq_builtins filtering to verify that the
    // completions shown to the user are appropriate for the pipe-context type.

    #[test]
    fn string_pipe_context_shows_string_functions_only() {
        use serde_json::json;
        let input = json!({"name": "hello"});
        let jq_type = detect_pipe_prefix_type(&input, ".name");
        assert_eq!(jq_type, "string");

        let token = ""; // no prefix typed yet — show everything for this type
        let completions = completions::jq_builtins::get_completions(token, Some(&jq_type));

        // String functions must appear
        for expected in &["ascii_upcase", "ascii_downcase", "split", "test", "ltrimstr"] {
            assert!(completions.iter().any(|c| &c.label == expected),
                "{} must be suggested for string input", expected);
        }
        // Number-only functions must NOT appear
        for excluded in &["floor", "ceil", "sqrt", "log", "exp"] {
            assert!(!completions.iter().any(|c| &c.label == excluded),
                "{} must NOT be suggested for string input", excluded);
        }
    }

    #[test]
    fn number_pipe_context_shows_math_functions() {
        use serde_json::json;
        let input = json!({"score": 9.7});
        let jq_type = detect_pipe_prefix_type(&input, ".score");
        assert_eq!(jq_type, "number");

        let completions = completions::jq_builtins::get_completions("", Some(&jq_type));

        for expected in &["floor", "ceil", "round", "sqrt", "fabs", "log"] {
            assert!(completions.iter().any(|c| &c.label == expected),
                "{} must be suggested for number input", expected);
        }
        for excluded in &["ascii_upcase", "split", "ltrimstr", "to_entries"] {
            assert!(!completions.iter().any(|c| &c.label == excluded),
                "{} must NOT be suggested for number input", excluded);
        }
    }

    #[test]
    fn array_pipe_context_shows_array_functions() {
        use serde_json::json;
        let input = json!({"tags": ["a", "b", "c"]});
        let jq_type = detect_pipe_prefix_type(&input, ".tags");
        assert_eq!(jq_type, "array");

        let completions = completions::jq_builtins::get_completions("", Some(&jq_type));

        for expected in &["sort", "reverse", "map", "flatten", "unique", "first", "last"] {
            assert!(completions.iter().any(|c| &c.label == expected),
                "{} must be suggested for array input", expected);
        }
        // to_entries is valid for arrays too (gives [{key:0,value:…}])
        for excluded in &["ascii_upcase", "floor", "ltrimstr", "strptime"] {
            assert!(!completions.iter().any(|c| &c.label == excluded),
                "{} must NOT be suggested for array input", excluded);
        }
    }

    #[test]
    fn token_prefix_applied_within_type_context() {
        use serde_json::json;
        let input = json!({"name": "hello"});
        let jq_type = detect_pipe_prefix_type(&input, ".name");

        // Typing "asc" after `.name | ` — only ascii_* should appear
        let completions = completions::jq_builtins::get_completions("asc", Some(&jq_type));
        assert!(completions.iter().any(|c| c.label == "ascii_upcase"),
            "ascii_upcase must appear for 'asc' token with string type");
        assert!(completions.iter().any(|c| c.label == "ascii_downcase"));
        assert!(completions.iter().all(|c| c.label.starts_with("asc")),
            "all results must match 'asc' prefix: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>());
    }

    // ── Backspace erasing scenarios ───────────────────────────────────────────
    // After the user accepts a completion and starts erasing, the stale LSP
    // cache filtered by the reduced token must still surface the correct item.

    #[test]
    fn backspace_mid_word_still_shows_completion() {
        // Accepted: ".name | ascii_upcase"
        // User erases back to: ".name | ascii_upcas" (missing final 'e')
        let cache = vec![item("ascii_upcase"), item("ascii_downcase")];
        let result = build_lsp_suggestions(&cache, "ascii_upcas", ".name | ");
        assert_eq!(result.len(), 1, "only ascii_upcase matches 'ascii_upcas'");
        assert_eq!(result[0].label, "ascii_upcase");
        assert_eq!(result[0].insert_text, ".name | ascii_upcase",
            "accept must restore the full query");
    }

    #[test]
    fn backspace_to_prefix_shows_all_matching() {
        // Erased all the way back to ".name | asc"
        let cache = vec![item("ascii_upcase"), item("ascii_downcase"), item("split")];
        let result = build_lsp_suggestions(&cache, "asc", ".name | ");
        assert_eq!(result.len(), 2, "ascii_upcase and ascii_downcase start with 'asc'");
        assert!(result.iter().all(|c| c.insert_text.starts_with(".name | ascii_")));
    }

    #[test]
    fn backspace_to_empty_token_shows_all_cached() {
        // Erased back to ".name | " — empty token, all cache entries should appear
        let cache = vec![item("ascii_upcase"), item("split"), item("length")];
        let result = build_lsp_suggestions(&cache, "", ".name | ");
        assert_eq!(result.len(), 3, "empty token must show all cached completions");
        assert!(result.iter().all(|c| c.insert_text.starts_with(".name | ")),
            "all insert_texts must include pipe prefix");
    }

    // ── Insert-text end-to-end for builtin accepts ────────────────────────────
    // When the user selects a builtin completion while in a pipe context, the
    // accepted text must be the full query (prefix + function), not just the
    // function name.

    #[test]
    fn builtin_insert_text_includes_pipe_prefix() {
        use serde_json::json;
        let input = json!({"value": 9.5});
        let jq_type = detect_pipe_prefix_type(&input, ".value");
        let query = ".value | f";
        let prefix = lsp_pipe_prefix(query); // ".value | "
        let token  = current_token(query);   // "f"

        let builtins = completions::jq_builtins::get_completions(token, Some(&jq_type));
        // Patch insert_text as main_loop does
        let with_prefix: Vec<_> = builtins.into_iter()
            .map(|c| format!("{}{}", prefix, c.insert_text))
            .collect();

        assert!(with_prefix.iter().any(|s| s == ".value | floor"),
            "accepting 'floor' must produce '.value | floor', got: {:?}", with_prefix);
        assert!(with_prefix.iter().any(|s| s == ".value | fabs"));
        // No string-only function should appear
        assert!(!with_prefix.iter().any(|s| s.contains("ascii_upcase")),
            "ascii_upcase must not appear for number context");
    }

    // ── cursor_col_after_accept ───────────────────────────────────────────────

    #[test]
    fn cursor_col_no_parens_lands_at_end() {
        // Functions without parameters: cursor goes to end.
        assert_eq!(cursor_col_after_accept("ascii_upcase"), 12);
        assert_eq!(cursor_col_after_accept("length"), 6);
        assert_eq!(cursor_col_after_accept("floor"), 5);
    }

    #[test]
    fn cursor_col_with_parens_lands_after_opening_quote() {
        // split(",") → '("' starts at index 5 → cursor at 7 (after `("`).
        assert_eq!(cursor_col_after_accept("split(\",\")"), 7);
        // ltrimstr("") → '("' at index 8 → cursor at 10.
        assert_eq!(cursor_col_after_accept("ltrimstr(\"\")"), 10);
        // test("") → '("' at index 4 → cursor at 6.
        assert_eq!(cursor_col_after_accept("test(\"\")"), 6);
    }

    #[test]
    fn cursor_col_with_pipe_prefix_uses_last_paren() {
        // Single function in prefix — same as before.
        // ".config | split(\",\")" → last '("' at index 16 → cursor at 18.
        let text = ".config | split(\",\")";
        let paren_quote_pos = text.rfind("(\"").unwrap();
        assert_eq!(cursor_col_after_accept(text), (paren_quote_pos + 2) as u16);
    }

    #[test]
    fn cursor_col_multi_pipe_lands_in_last_function() {
        // Multi-pipe: cursor must land inside the *last* function, not the first.
        // ".name | split(\",\") | ltrimstr(\"\")"
        //                         last '("' is inside ltrimstr, not split.
        let text = ".name | split(\",\") | ltrimstr(\"\")";
        let last_pos = text.rfind("(\"").unwrap(); // inside ltrimstr
        assert_eq!(cursor_col_after_accept(text), (last_pos + 2) as u16);
        // Verify it is NOT pointing into split's parens (earlier position).
        let first_pos = text.find("(\"").unwrap();
        assert!(last_pos > first_pos, "rfind must return the later position");
    }

    #[test]
    fn builtin_insert_text_for_string_context() {
        use serde_json::json;
        let input = json!({"label": "hello"});
        let jq_type = detect_pipe_prefix_type(&input, ".label");
        let query = ".label | ascii_up";
        let prefix = lsp_pipe_prefix(query); // ".label | "
        let token  = current_token(query);   // "ascii_up"

        let builtins = completions::jq_builtins::get_completions(token, Some(&jq_type));
        let with_prefix: Vec<_> = builtins.into_iter()
            .map(|c| format!("{}{}", prefix, c.insert_text))
            .collect();

        assert!(with_prefix.iter().any(|s| s == ".label | ascii_upcase"),
            "accepting ascii_upcase must give '.label | ascii_upcase', got: {:?}", with_prefix);
    }
}
