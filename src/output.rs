use anyhow::{Context, Result};
use jaq_fmts::Format;
use jqpp::app::App;
use jqpp::executor::{Executor, val_to_json};
use std::path::Path;

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

/// Parse a file using the format determined by its extension.
/// Falls through to the JSON-first path for unrecognised extensions.
pub fn parse_file_by_format(data: &[u8], path: &Path) -> Result<serde_json::Value> {
    match Format::determine(path) {
        None | Some(Format::Json) => parse_input_as_json_or_string(data),
        Some(fmt) => parse_format(data, fmt)
            .with_context(|| format!("Failed to parse {:?} as {:?}", path, fmt)),
    }
}

/// Parse stdin: try JSON, then YAML, then fall back to raw string.
pub fn parse_stdin_with_yaml_fallback(data: &[u8]) -> Result<serde_json::Value> {
    if let Ok(json) = serde_json::from_slice(data) {
        return Ok(json);
    }
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(val) = parse_yaml_str(s)
    {
        return Ok(val);
    }
    parse_input_as_json_or_string(data)
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

fn parse_format(data: &[u8], fmt: Format) -> Result<serde_json::Value> {
    match fmt {
        Format::Yaml => {
            let s = std::str::from_utf8(data).context("YAML file is not valid UTF-8")?;
            parse_yaml_str(s)
        }
        Format::Toml => {
            let s = std::str::from_utf8(data).context("TOML file is not valid UTF-8")?;
            let val = jaq_fmts::read::toml::parse(s).map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(val_to_json(val))
        }
        Format::Xml => {
            let s = std::str::from_utf8(data).context("XML file is not valid UTF-8")?;
            let vals: Vec<_> = jaq_fmts::read::xml::parse_many(s)
                .map(|r| r.map(val_to_json).map_err(|e| anyhow::anyhow!("{e}")))
                .collect::<Result<Vec<_>>>()?;
            Ok(collect_vals(vals))
        }
        Format::Cbor => {
            let vals: Vec<_> = jaq_fmts::read::cbor::parse_many(data)
                .map(|r| r.map(val_to_json).map_err(|e| anyhow::anyhow!("{e}")))
                .collect::<Result<Vec<_>>>()?;
            Ok(collect_vals(vals))
        }
        Format::Csv => {
            let rows: Vec<_> = jaq_fmts::read::tabular::read_csv(
                data.iter().copied().map(Ok::<u8, std::io::Error>),
            )
            .map(|r| r.map(val_to_json).map_err(|e| anyhow::anyhow!("{e}")))
            .collect::<Result<Vec<_>>>()?;
            Ok(serde_json::Value::Array(rows))
        }
        Format::Tsv => {
            let rows: Vec<_> = jaq_fmts::read::tabular::read_tsv(
                data.iter().copied().map(Ok::<u8, std::io::Error>),
            )
            .map(|r| r.map(val_to_json).map_err(|e| anyhow::anyhow!("{e}")))
            .collect::<Result<Vec<_>>>()?;
            Ok(serde_json::Value::Array(rows))
        }
        // Json is handled before this function is called; Raw/Raw0 have no file extension
        _ => parse_input_as_json_or_string(data),
    }
}

fn parse_yaml_str(s: &str) -> Result<serde_json::Value> {
    let docs: Vec<_> = jaq_fmts::read::yaml::parse_many(s)
        .map(|r| r.map(val_to_json).map_err(|e| anyhow::anyhow!("{e}")))
        .collect::<Result<Vec<_>>>()?;
    if docs.is_empty() {
        return Err(anyhow::anyhow!("YAML input is empty"));
    }
    Ok(collect_vals(docs))
}

fn collect_vals(mut docs: Vec<serde_json::Value>) -> serde_json::Value {
    if docs.len() == 1 {
        docs.remove(0)
    } else {
        serde_json::Value::Array(docs)
    }
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

    #[test]
    fn yaml_mapping_parses_to_json_object() {
        let data = b"name: alice\nage: 30\n";
        let result = parse_file_by_format(data, Path::new("config.yaml")).unwrap();
        assert_eq!(result, serde_json::json!({"name": "alice", "age": 30}));
    }

    #[test]
    fn yml_sequence_parses_to_json_array() {
        let data = b"- 1\n- 2\n- 3\n";
        let result = parse_file_by_format(data, Path::new("list.yml")).unwrap();
        assert_eq!(result, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn malformed_yaml_file_returns_err() {
        let data = b"name: [unclosed\n";
        assert!(parse_file_by_format(data, Path::new("bad.yaml")).is_err());
    }

    #[test]
    fn toml_file_parses_to_json_object() {
        let data = b"name = \"alice\"\nage = 30\n";
        let result = parse_file_by_format(data, Path::new("config.toml")).unwrap();
        assert_eq!(result, serde_json::json!({"name": "alice", "age": 30}));
    }

    #[test]
    fn xml_file_parses_without_error() {
        let data = b"<root><item>hello</item></root>";
        let result = parse_file_by_format(data, Path::new("doc.xml")).unwrap();
        assert!(result.is_object());
    }

    #[test]
    fn csv_file_parses_to_array_of_row_arrays() {
        let data = b"1,alice,true\n2,bob,false\n";
        let result = parse_file_by_format(data, Path::new("data.csv")).unwrap();
        assert_eq!(
            result,
            serde_json::json!([[1, "alice", true], [2, "bob", false]])
        );
    }

    #[test]
    fn tsv_file_parses_to_array_of_row_arrays() {
        let data = b"1\talice\n2\tbob\n";
        let result = parse_file_by_format(data, Path::new("data.tsv")).unwrap();
        assert_eq!(result, serde_json::json!([[1, "alice"], [2, "bob"]]));
    }

    #[test]
    fn json_file_is_unaffected() {
        let data = b"{\"x\": 1}";
        let result = parse_file_by_format(data, Path::new("data.json")).unwrap();
        assert_eq!(result, serde_json::json!({"x": 1}));
    }

    #[test]
    fn stdin_yaml_parses_to_json_when_json_fails() {
        let data = b"name: alice\nage: 30\n";
        let result = parse_stdin_with_yaml_fallback(data).unwrap();
        assert_eq!(result, serde_json::json!({"name": "alice", "age": 30}));
    }

    #[test]
    fn stdin_plain_text_falls_back_to_string() {
        let result = parse_stdin_with_yaml_fallback(b"hello").unwrap();
        assert_eq!(result, serde_json::json!("hello"));
    }

    #[test]
    fn stdin_valid_json_uses_json_path() {
        let result = parse_stdin_with_yaml_fallback(b"{\"a\":1}").unwrap();
        assert_eq!(result, serde_json::json!({"a": 1}));
    }
}
