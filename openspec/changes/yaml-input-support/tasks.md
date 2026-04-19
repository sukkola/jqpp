## 1. Dependency

- [x] 1.1 Add `jaq-fmts = { version = "0.1", default-features = false, features = ["yaml", "toml", "xml", "cbor", "tabular"] }` to `[dependencies]` in `Cargo.toml`
- [x] 1.2 Run `cargo check` / verify `Cargo.lock` updates cleanly

## 2. Val → JSON Conversion

- [x] 2.1 Rename `val_to_value` in `src/executor.rs` to `val_to_json` and make it `pub` so `output.rs` can reuse it

## 3. Multi-Format Parsing Helper

- [x] 3.1 Add `parse_file_by_format(data: &[u8], path: &Path) -> Result<serde_json::Value>` in `src/output.rs` that dispatches via `jaq_fmts::Format::determine(path)` to the appropriate `jaq_fmts::read::*` parser; unrecognised extensions fall through to `parse_input_as_json_or_string`
- [x] 3.2 Add `parse_stdin_with_yaml_fallback(data: &[u8]) -> Result<serde_json::Value>` in `src/output.rs` that tries JSON, then YAML, then the existing string fallback

## 4. File Input Dispatch

- [x] 4.1 In `load_inputs` (`src/main.rs`), call `parse_file_by_format(data, path)` instead of `parse_input_as_json_or_string(data)` for file reads
- [x] 4.2 In the stdin branch of `load_inputs`, call `parse_stdin_with_yaml_fallback` instead of `parse_input_as_json_or_string`

## 5. Tests

- [x] 5.1 Unit test: `.yaml` file with a YAML mapping parses to the expected JSON object
- [x] 5.2 Unit test: `.yml` file with a YAML sequence parses to the expected JSON array
- [x] 5.3 Unit test: malformed `.yaml` file returns `Err` (not a string fallback)
- [x] 5.4 Unit test: `.toml` file parses to the expected JSON object
- [x] 5.5 Unit test: `.xml` file parses without error (basic smoke test)
- [x] 5.6 Unit test: `.csv` file parses to an array of row-arrays
- [x] 5.7 Unit test: `.tsv` file parses to an array of row-arrays
- [x] 5.8 Unit test: mixed `.json` + `.yaml` multi-file invocation merges into a two-element JSON array
- [x] 5.9 Unit test: stdin YAML content is parsed to JSON value when JSON parse fails
- [x] 5.10 Unit test: stdin plain text (non-JSON, non-YAML) still falls back to JSON string
- [x] 5.11 Unit test: `.json` file is unaffected (still uses JSON path, not format dispatch)
