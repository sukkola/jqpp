## Why

jaq (the Rust jq implementation powering this tool) uses `jaq-fmts` to support multiple input formats. jqpp currently only parses JSON and treats other files as raw strings. Users working with YAML, TOML, XML, CSV, TSV, and CBOR data have to pre-convert their files to JSON, which breaks the tool's zero-friction goal.

## What Changes

- Files with recognised extensions (`.yaml`, `.yml`, `.toml`, `.xml`, `.xhtml`, `.cbor`, `.csv`, `.tsv`) are automatically parsed using `jaq-fmts` and converted to `serde_json::Value` for querying.
- Format is detected via `jaq_fmts::Format::determine(path)` — same logic jaq itself uses.
- Stdin pipe content that fails JSON parse is also attempted as YAML before falling back to a raw string value.
- Non-JSON, non-YAML stdin content continues to fall back to a JSON string value (existing behaviour).
- CSV and TSV files produce an array of row-arrays (consistent with `jaq-fmts` tabular output).

## Capabilities

### New Capabilities

- `multi-format-input`: Detect and parse files in any format supported by `jaq-fmts` into `serde_json::Value` so they can be queried with jq expressions, identical to how JSON inputs are handled.

### Modified Capabilities

- `multi-file-input`: Format-aware files are now valid members of a multi-file invocation; the merge-to-array behaviour is unchanged but the per-file parser dispatches on format instead of always using the JSON path.

## Impact

- **Dependency**: Add `jaq-fmts = { version = "0.1", default-features = false, features = ["yaml", "toml", "xml", "cbor", "tabular"] }` to `Cargo.toml`. No other new dependencies — `jaq-fmts` reuses `jaq-json` already present.
- **Code**: `load_inputs` in `src/main.rs` calls a new `parse_file_by_format` helper (in `src/output.rs`) instead of always using `parse_input_as_json_or_string`. Stdin gains a YAML fallback attempt.
- **No breaking changes**: all existing JSON and raw-string paths are preserved.
- **Tests**: new unit tests per format covering single-file, multi-file merge, and stdin YAML cases.
