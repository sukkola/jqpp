mod accept;
mod handlers;
mod hints;
mod loop_state;
mod mouse;
mod output;
mod suggestions;
mod terminal;

use jqpp::app::App;
use jqpp::completions::lsp::{LspMessage, LspProvider};
use jqpp::config;
use jqpp::executor::Executor;
use jqpp::keymap;
use jqpp::ui;

use anyhow::{Context, Result};
use clap::{ArgGroup, Parser};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::loop_state::LoopState;
use crate::output::{
    OutputMode, output_mode_from_args, parse_input_as_json_or_string, selected_output,
};
use crate::suggestions::{handle_finished_computes, handle_lsp_message, run_debounced_compute};
use crate::terminal::{TerminalGuard, TtyWriter, get_tty_handle, lsp_on_path, setup_panic_hook};

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

fn actual_main(args: Args) -> Result<()> {
    setup_panic_hook(args.debug);
    let output_mode = output_mode_from_args(args.print_output, args.print_query, args.print_input);

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
    mut lsp_provider: Option<LspProvider>,
    lsp_rx: &mut mpsc::Receiver<LspMessage>,
    keymap: &keymap::Keymap,
) -> Result<()> {
    let mut state = LoopState::new();
    state.debounce_duration = Duration::from_millis(80);

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

    if let (Some(m), Some(at)) = (app.footer_message.take(), app.footer_message_at.take()) {
        state.footer_message = Some((m, at));
    }

    while app.running {
        handle_finished_computes(app, &mut state).await;

        terminal
            .draw(|f| ui::draw(f, app, keymap))
            .context("Failed to draw TUI frame")?;

        while let Ok(msg) = lsp_rx.try_recv() {
            handle_lsp_message(app, &mut state, msg);
        }

        if ratatui::crossterm::event::poll(Duration::from_millis(8))
            .context("Failed to poll for terminal events")?
        {
            state
                .poll_and_process_events(terminal, app, keymap, &mut key_log)
                .await?;
        }

        run_debounced_compute(app, &mut state, &mut lsp_provider).await;

        if let Some((ref msg, start)) = state.footer_message {
            let timeout = if msg.starts_with("Config") { 5 } else { 2 };
            if start.elapsed() >= Duration::from_secs(timeout) {
                state.footer_message = None;
            }
        }
        app.footer_message = state.footer_message.as_ref().map(|(m, _)| m.clone());
    }

    if let Some(mut lsp) = lsp_provider {
        let _ = lsp.shutdown().await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
}
