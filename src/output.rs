use anyhow::{Context, Result};
use jqpp::app::App;
use jqpp::executor::Executor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Result,
    Query,
    Input,
}

pub fn copy_text_to_clipboard(text: String) {
    std::thread::spawn(move || {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            return;
        };

        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
        ))]
        {
            use std::time::{Duration, Instant};
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

pub fn output_mode_from_args(
    print_output: bool,
    print_query: bool,
    print_input: bool,
) -> Option<OutputMode> {
    if print_output {
        Some(OutputMode::Result)
    } else if print_query {
        Some(OutputMode::Query)
    } else if print_input {
        Some(OutputMode::Input)
    } else {
        None
    }
}

pub fn selected_output(app: &App<'_>, mode: OutputMode) -> Option<String> {
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

pub fn parse_input_as_json_or_string(input_data: &[u8]) -> Result<serde_json::Value> {
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

pub fn right_pane_copy_text(app: &App<'_>) -> String {
    if !app.results.is_empty() {
        Executor::format_results(&app.results, app.raw_output)
    } else if let Some(ref err) = app.error {
        err.clone()
    } else {
        Executor::format_results(&app.results, app.raw_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jqpp::app::App;

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
}
