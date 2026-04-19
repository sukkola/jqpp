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
    OutputMode, output_mode_from_args, parse_file_by_format, parse_stdin_with_yaml_fallback,
    selected_output,
};
use crate::suggestions::{handle_finished_computes, handle_lsp_message, run_debounced_compute};
use crate::terminal::{TerminalGuard, TtyWriter, get_tty_handle, lsp_on_path, setup_panic_hook};

type LoadInputsResult = (
    Vec<u8>,
    Option<serde_json::Value>,
    Vec<String>,
    Option<jaq_fmts::Format>,
);

#[derive(Parser, Debug)]
#[command(version)]
#[command(group(
    ArgGroup::new("output")
        .args(["print_output", "print_query", "print_input"])
        .multiple(false)
))]
struct Args {
    /// Positional [files]
    #[arg(num_args(0..))]
    files: Vec<PathBuf>,

    /// Initial query string
    #[arg(long)]
    query: Option<String>,

    /// Initial cursor column (0-based from start; negative counts from end)
    #[arg(long, allow_hyphen_values = true)]
    cursor: Option<i32>,

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

fn actual_main(mut args: Args) -> Result<()> {
    setup_panic_hook(args.debug);
    let output_mode = output_mode_from_args(args.print_output, args.print_query, args.print_input);

    let stdin_is_terminal = io::stdin().is_terminal();

    let (display_raw, json_opt, labels, source_format) =
        load_inputs(&args.files, stdin_is_terminal)?;

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

    let executor = if let Some(json_input) = json_opt {
        let source_label = if labels.is_empty() {
            "stdin".to_string()
        } else {
            let joined = labels.join(", ");
            if joined.len() > 60 {
                format!("{}…", &joined[..59])
            } else {
                joined
            }
        };

        if labels.len() > 1 && args.query.is_none() {
            args.query = Some(".[]".to_string());
        }

        Some(Executor {
            raw_input: display_raw,
            json_input,
            source_label,
            source_format,
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

fn load_inputs(files: &[PathBuf], stdin_is_terminal: bool) -> Result<LoadInputsResult> {
    let mut input_values: Vec<serde_json::Value> = Vec::new();
    let mut labels = Vec::new();
    let mut first_display_raw: Option<Vec<u8>> = None;
    let mut first_source_format: Option<jaq_fmts::Format> = None;

    if !stdin_is_terminal {
        let mut stdin_data = Vec::new();
        io::stdin()
            .read_to_end(&mut stdin_data)
            .context("Failed to read from stdin pipe")?;
        if !stdin_data.is_empty() {
            let val = parse_stdin_with_yaml_fallback(&stdin_data)?;
            if first_display_raw.is_none() {
                let is_json = serde_json::from_slice::<serde_json::Value>(&stdin_data).is_ok();
                first_source_format = if !is_json && !val.is_string() {
                    Some(jaq_fmts::Format::Yaml)
                } else {
                    None
                };
                first_display_raw = Some(stdin_data);
            }
            input_values.push(val);
            labels.push("stdin".to_string());
        }
    }

    for f_path in files {
        let data = std::fs::read(f_path).context(format!("Failed to read file: {:?}", f_path))?;
        let val = parse_file_by_format(&data, f_path)?;
        let fmt = jaq_fmts::Format::determine(f_path);
        let is_json_format = fmt
            .map(|f| matches!(f, jaq_fmts::Format::Json))
            .unwrap_or(true);
        if first_display_raw.is_none() {
            first_source_format = if is_json_format { None } else { fmt };
            first_display_raw = Some(data);
        }
        input_values.push(val);
        labels.push(
            f_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| f_path.to_string_lossy().to_string()),
        );
    }

    if input_values.is_empty() {
        return Ok((Vec::new(), None, Vec::new(), None));
    }

    if input_values.len() == 1 {
        let val = input_values.remove(0);
        Ok((
            first_display_raw.unwrap(),
            Some(val),
            labels,
            first_source_format,
        ))
    } else {
        let merged = serde_json::Value::Array(input_values);
        let raw = serde_json::to_vec(&merged)?;
        Ok((raw, Some(merged), labels, None))
    }
}

fn resolve_cursor(cursor_arg: Option<i32>, query_len: usize) -> usize {
    if let Some(col) = cursor_arg {
        if col >= 0 {
            (col as usize).min(query_len)
        } else {
            query_len.saturating_sub((-col) as usize)
        }
    } else {
        query_len
    }
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

    if let Some(q) = args.query.as_ref()
        && !q.is_empty()
    {
        app.query_input.textarea.insert_str(q);
        let query_len = q.chars().count();
        let resolved = resolve_cursor(args.cursor, query_len);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, resolved as u16));
    }

    if std::env::var("JQPP_SKIP_TTY_CHECK").is_ok() {
        if output_mode.is_some() {
            if let Some(ref exec) = app.executor {
                let q = args.query.as_deref().unwrap_or(".");
                if let Ok(results) = Executor::execute(q, &exec.json_input) {
                    app.results = results;
                    app.error = None;
                    app.raw_output = false;
                }
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
        let initial_query = app.query_input.textarea.lines()[0].clone();
        let q = if initial_query.trim().is_empty() {
            ".".to_string()
        } else {
            initial_query
        };
        if let Ok(Ok(results)) =
            tokio::task::spawn_blocking(move || Executor::execute(&q, &input)).await
        {
            app.results = results;
        }
    }

    if !app.query_input.textarea.lines()[0].is_empty() {
        let query_prefix = crate::suggestions::current_query_prefix(app);
        app.query_input.suggestions = crate::suggestions::compute_suggestions(
            &query_prefix,
            app.executor.as_ref().map(|e| &e.json_input),
            &state.lsp_completions,
            state.cached_pipe_type.as_deref(),
        );
        app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
        state.suggestion_active = app.query_input.show_suggestions;
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
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn cursor_positive_within_bounds_unchanged() {
        assert_eq!(resolve_cursor(Some(2), 5), 2);
    }

    #[test]
    fn cursor_positive_clamped_to_query_length() {
        assert_eq!(resolve_cursor(Some(999), 3), 3);
    }

    #[test]
    fn cursor_positive_zero_for_empty_query() {
        assert_eq!(resolve_cursor(Some(0), 0), 0);
    }

    #[test]
    fn cursor_negative_minus_one_is_end() {
        // Semantic: -1 is index of last char, so cursor is placed BEFORE it?
        // Task 7.4 second part says resolved to 3 for len 4.
        assert_eq!(resolve_cursor(Some(-1), 4), 3);
    }

    #[test]
    fn cursor_negative_minus_query_len_is_start() {
        // If len 4, -4 is index 0.
        assert_eq!(resolve_cursor(Some(-4), 4), 0);
    }

    #[test]
    fn cursor_negative_more_than_query_len_clamps_to_zero() {
        assert_eq!(resolve_cursor(Some(-999), 3), 0);
    }

    #[test]
    fn cursor_negative_mid_query() {
        assert_eq!(resolve_cursor(Some(-3), 10), 7);
    }

    #[test]
    fn single_file_not_wrapped_in_array() {
        let temp = std::env::temp_dir().join("single_input_test.json");
        std::fs::write(&temp, b"{\"a\":1}").unwrap();
        let (raw, _json_opt, labels, _fmt) =
            load_inputs(std::slice::from_ref(&temp), true).unwrap();
        assert_eq!(labels, vec!["single_input_test.json"]);
        assert_eq!(String::from_utf8_lossy(&raw), "{\"a\":1}");
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn two_files_merged_into_array() {
        let t1 = std::env::temp_dir().join("two_files_1.json");
        let t2 = std::env::temp_dir().join("two_files_2.json");
        std::fs::write(&t1, b"1").unwrap();
        std::fs::write(&t2, b"2").unwrap();
        let (raw, _, _, _) = load_inputs(&[t1.clone(), t2.clone()], true).unwrap();
        let val: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(val, json!([1, 2]));
        let _ = std::fs::remove_file(t1);
        let _ = std::fs::remove_file(t2);
    }

    #[test]
    fn non_json_file_becomes_string_in_array() {
        let t1 = std::env::temp_dir().join("non_json_1.json");
        let t2 = std::env::temp_dir().join("non_json_2.txt");
        std::fs::write(&t1, b"1").unwrap();
        std::fs::write(&t2, b"hello").unwrap();
        let (raw, _, _, _) = load_inputs(&[t1.clone(), t2.clone()], true).unwrap();
        let val: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(val, json!([1, "hello"]));

        let _ = std::fs::remove_file(t1);
        let _ = std::fs::remove_file(t2);
    }

    #[test]
    fn mixed_types_preserved_in_merged_array() {
        let t1 = std::env::temp_dir().join("mixed_1.json");
        let t2 = std::env::temp_dir().join("mixed_2.json");
        let t3 = std::env::temp_dir().join("mixed_3.json");
        std::fs::write(&t1, b"{\"x\":1}").unwrap();
        std::fs::write(&t2, b"[1,2]").unwrap();
        std::fs::write(&t3, b"\"foo\"").unwrap();
        let (raw, _, _, _) = load_inputs(&[t1.clone(), t2.clone(), t3.clone()], true).unwrap();
        let val: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(val, json!([{"x":1}, [1,2], "foo"]));
        let _ = std::fs::remove_file(t1);
        let _ = std::fs::remove_file(t2);
        let _ = std::fs::remove_file(t3);
    }

    #[test]
    fn duplicate_file_produces_two_entries() {
        let t1 = std::env::temp_dir().join("dup_1.json");
        std::fs::write(&t1, b"1").unwrap();
        let (raw, _, labels, _) = load_inputs(&[t1.clone(), t1.clone()], true).unwrap();
        let val: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(val, json!([1, 1]));
        assert_eq!(labels, vec!["dup_1.json", "dup_1.json"]);
        let _ = std::fs::remove_file(t1);
    }

    #[test]
    fn source_label_for_two_files() {
        let t1 = std::env::temp_dir().join("label_1.json");
        let t2 = std::env::temp_dir().join("label_2.json");
        std::fs::write(&t1, b"1").unwrap();
        std::fs::write(&t2, b"2").unwrap();
        let (_, _, labels, _) = load_inputs(&[t1.clone(), t2.clone()], true).unwrap();
        assert_eq!(labels, vec!["label_1.json", "label_2.json"]);
        let _ = std::fs::remove_file(t1);
        let _ = std::fs::remove_file(t2);
    }

    #[test]
    fn default_query_is_dot_slice_for_two_inputs() {
        let t1 = std::env::temp_dir().join("dq_1.json");
        let t2 = std::env::temp_dir().join("dq_2.json");
        std::fs::write(&t1, b"1").unwrap();
        std::fs::write(&t2, b"2").unwrap();

        let args = Args {
            files: vec![t1, t2],
            query: None,
            cursor: None,
            no_lsp: false,
            debug: false,
            config: None,
            print_config: false,
            print_output: false,
            print_query: false,
            print_input: false,
        };

        let stdin_is_terminal = true;
        let (_, _, labels, _) = load_inputs(&args.files, stdin_is_terminal).unwrap();
        let mut final_args = args;
        if labels.len() > 1 && final_args.query.is_none() {
            final_args.query = Some(".[]".to_string());
        }
        assert_eq!(final_args.query.as_deref(), Some(".[]"));
    }

    #[test]
    fn explicit_query_overrides_dot_slice_default() {
        let t1 = std::env::temp_dir().join("eq_1.json");
        let t2 = std::env::temp_dir().join("eq_2.json");
        std::fs::write(&t1, b"1").unwrap();
        std::fs::write(&t2, b"2").unwrap();

        let args = Args {
            files: vec![t1, t2],
            query: Some(".[] | .name".to_string()),
            cursor: None,
            no_lsp: false,
            debug: false,
            config: None,
            print_config: false,
            print_output: false,
            print_query: false,
            print_input: false,
        };

        let stdin_is_terminal = true;
        let (_, _, labels, _) = load_inputs(&args.files, stdin_is_terminal).unwrap();
        let mut final_args = args;
        if labels.len() > 1 && final_args.query.is_none() {
            final_args.query = Some(".[]".to_string());
        }
        assert_eq!(final_args.query.as_deref(), Some(".[] | .name"));
    }

    #[test]
    fn yaml_file_mapping_is_parsed_to_json_object() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("input.yaml");
        std::fs::write(&file, b"name: alice\nage: 30\n").unwrap();

        let (raw, json_opt, labels, fmt) = load_inputs(&[file], true).unwrap();
        assert_eq!(labels, vec!["input.yaml"]);
        assert_eq!(raw, b"name: alice\nage: 30\n");
        assert!(matches!(fmt, Some(jaq_fmts::Format::Yaml)));
        assert_eq!(json_opt.unwrap(), json!({"name": "alice", "age": 30}));
    }

    #[test]
    fn yml_file_sequence_is_parsed_to_json_array() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("input.yml");
        std::fs::write(&file, b"- one\n- two\n- three\n").unwrap();

        let (raw, json_opt, labels, fmt) = load_inputs(&[file], true).unwrap();
        assert_eq!(labels, vec!["input.yml"]);
        assert_eq!(raw, b"- one\n- two\n- three\n");
        assert!(matches!(fmt, Some(jaq_fmts::Format::Yaml)));
        assert_eq!(json_opt.unwrap(), json!(["one", "two", "three"]));
    }

    #[test]
    fn malformed_yaml_file_returns_err() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("broken.yaml");
        std::fs::write(&file, b"name: [alice\n").unwrap();

        let err = load_inputs(&[file], true).unwrap_err();
        assert!(!err.to_string().is_empty(), "unexpected error: {err}");
    }

    #[test]
    fn mixed_json_and_yaml_files_merge_into_array() {
        let dir = tempdir().unwrap();
        let json_file = dir.path().join("data.json");
        let yaml_file = dir.path().join("config.yaml");
        std::fs::write(&json_file, b"{\"a\":1}").unwrap();
        std::fs::write(&yaml_file, b"b: 2\n").unwrap();

        let (raw, _, _, _) = load_inputs(&[json_file, yaml_file], true).unwrap();
        let val: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(val, json!([{"a": 1}, {"b": 2}]));
    }

    #[test]
    fn json_file_path_remains_unaffected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("still-json.json");
        std::fs::write(&file, b"{\"ok\":true}").unwrap();

        let (_raw, json_opt, labels, _fmt) = load_inputs(&[file], true).unwrap();
        let val = json_opt.unwrap();
        assert_eq!(labels, vec!["still-json.json"]);
        assert_eq!(val, json!({"ok": true}));
    }
}
