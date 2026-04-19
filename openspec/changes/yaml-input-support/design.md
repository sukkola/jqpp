## Context

jqpp reads input from files or stdin, parses them as JSON (with a raw-string fallback), and feeds the result to the jaq executor. The entry point is `parse_input_as_json_or_string` in `src/output.rs`, called from `load_inputs` in `src/main.rs`.

`jaq-fmts` is the official multi-format companion crate to `jaq-core` and `jaq-json`, both of which this project already depends on. It provides a `Format` enum with an `determine(path)` helper that maps file extensions to formats, and per-format `parse_many`/`parse` functions that return `jaq_json::Val` — the same value type the executor already uses internally.

## Goals / Non-Goals

**Goals:**
- Parse files by extension using `jaq_fmts::Format::determine(path)`: yaml/yml → YAML, toml → TOML, xml/xhtml → XML, cbor → CBOR, csv → CSV, tsv → TSV.
- For stdin, attempt YAML parse after JSON fails, before falling back to raw string.
- Preserve all existing JSON and raw-string fallback behaviour unchanged.
- Reuse the `jaq-fmts` crate that jaq itself uses — no separate YAML-only dependency.

**Non-Goals:**
- YAML/TOML/XML output (jq queries always produce JSON output).
- `--from <format>` CLI flag for overriding format detection (future work).
- Detecting format from stdin content (beyond the YAML fallback heuristic).

## Decisions

### Decision 1: Use `jaq-fmts` instead of `serde_yaml`

**Choice**: Add `jaq-fmts = { version = "0.1", default-features = false, features = ["yaml", "toml", "xml", "cbor", "tabular"] }`.

**Rationale**: `jaq-fmts` is authored by the same maintainer as `jaq-core` and uses `jaq_json::Val` natively — the same internal value type the executor already works with. Using it means the conversion path is `Val → serde_json::Value` via the existing `val_to_value` logic already in `executor.rs`, with no intermediate types. It also gives us all other formats for free via the same code path, since `Format::determine()` already covers them.

**Alternative considered**: `serde_yaml` — adds a second dependency, only covers YAML, requires going `serde_yaml::Value → serde_json::Value` (either through JSON round-trip or manual mapping). More work for less coverage.

### Decision 2: Single dispatch function using `Format::determine()`

**Choice**: Add `parse_file_by_format(data: &[u8], path: &Path) -> Result<serde_json::Value>` in `src/output.rs`. It calls `jaq_fmts::Format::determine(path)` and dispatches to the appropriate `jaq_fmts::read::*` parser. Unrecognised extensions fall through to the existing `parse_input_as_json_or_string`.

**Rationale**: `Format::determine` is the same extension-to-format mapping jaq itself uses, so behaviour is consistent. A single dispatch function keeps `load_inputs` in `main.rs` clean — it just calls `parse_file_by_format` for files and `parse_stdin_with_yaml_fallback` for stdin.

### Decision 3: Multi-value formats (YAML multi-doc, XML multi-root, CBOR stream) collapse to array if >1 value

**Choice**: For formats that can return multiple values (YAML multi-document, XML multiple root elements, CBOR stream), collect all values. If exactly one value, use it directly; if more, wrap in `serde_json::Value::Array`. CSV/TSV always return an array of row-arrays.

**Rationale**: Consistent with the existing multi-file merge behaviour — single input is unwrapped, multiple inputs become an array. Avoids surprising behaviour for the common single-document case.

### Decision 4: YAML is the only stdin fallback format

**Choice**: For stdin (no extension), after JSON parse fails, attempt YAML parse. No attempt for other formats (TOML, XML, CBOR).

**Rationale**: YAML is the most common text format piped via stdin after JSON. TOML and XML are less common and distinguishable enough that a `--from` flag would be more appropriate. CBOR is binary and not suitable for heuristic detection.

### Decision 5: Expose `val_to_json` from `executor.rs`

**Choice**: Make `val_to_value` in `executor.rs` pub (renamed `val_to_json`) so `output.rs` can reuse it for `Val → serde_json::Value` conversion.

**Rationale**: Avoids duplicating the conversion logic which handles jaq-specific types (`BStr`, `TStr`, `BigInt`, `Dec`). The function is simple but has several edge-case branches that are best maintained in one place.

## Risks / Trade-offs

- **TOML key ordering**: `jaq-fmts` TOML parser sorts keys by source span position — matches file order, which is intuitive.
- **CSV/TSV always arrays**: A CSV file passed as the sole input will have `.[]` as the sensible first query, not `.`. The auto-`.[]` heuristic already fires for multi-file inputs; for single CSV files users just need to know to use `.[]`.  → Document in README / footer hint.
- **CBOR byte strings**: CBOR `!!binary` values become `Val::BStr`, which `val_to_json` converts to a lossy UTF-8 string. This is the existing behaviour in the executor for any byte string. → Acceptable for v1.
- **`jaq-fmts` dependency size**: ~200 KB additional binary size for all features. Negligible for a TUI tool.

## Migration Plan

No migration needed — purely additive. Existing invocations with JSON files or stdin JSON are unaffected.
