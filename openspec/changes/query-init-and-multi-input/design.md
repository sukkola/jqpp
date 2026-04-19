## Context

`App` holds a `QueryInput` (wrapping `tui_textarea::TextArea`) as the query bar. The textarea is initialised empty in `App::new()`; the first draw happens after `actual_main` calls `run()`. All input reading (files, stdin) happens before `App` is constructed. The `Args` struct in `main.rs` currently accepts one optional `file: Option<PathBuf>` positional.

`QueryInput.textarea` is a public field, so setting its content and cursor before the first draw requires only calling `textarea.insert_str()` and `textarea.move_cursor()` after constructing the `App` — no new library-crate API is needed.

Suggestion computation is triggered inside `main_loop` via `run_debounced_compute` on every edit, and by `compute_suggestions` directly. For a pre-filled query we need one initial compute before the first draw so that ghost text and suggestions are visible immediately.

## Goals / Non-Goals

**Goals:**
- `--query <expr>` sets the query bar text on startup, including immediate suggestion activation.
- `--cursor <col>` sets the 0-based character column of the initial cursor within that query.
- Multiple positional file arguments merge their parsed JSON values into a single array dataset.
- Stdin pipe combined with one or more file arguments is included in the merged array.
- Multi-file mode auto-fills `.[]` as the initial query when `--query` is not given.
- All existing single-file and stdin-pipe behaviour is preserved byte-for-byte.
- Thorough unit and integration test coverage of all new code paths.

**Non-Goals:**
- Glob expansion of file arguments (shell handles that).
- Streaming very large files — same size constraints as today.
- A `--cursor-line` flag (queries are always single-line; column is sufficient).
- Auto-detecting whether merged inputs should be an object instead of an array.

## Decisions

### D1: `file` becomes `Vec<PathBuf>` with `num_args(0..)`

Clap supports `#[arg(num_args(0..))]` on a `Vec<PathBuf>` field. Zero files → existing behaviour. One file → existing single-file path. Two or more → multi-file mode. Stdin pipe is additive in all cases.

Considered: a separate `--files` flag. Rejected because positional arguments are more ergonomic for typical shell usage (`jqpp a.json b.json c.json`).

### D2: Merge strategy — JSON array of parsed values

Each input (file or stdin chunk) is parsed as JSON (or as a string value if unparseable, same as today's single-input fallback). The merged `json_input` is `serde_json::Value::Array(vec![val1, val2, ...])`. The `raw_input` bytes in `Executor` become the serialised form of the array.

Considered: newline-delimited concatenation. Rejected because heterogeneous types (object, array, scalar) need a stable outer wrapper, and ndjson would require a different parser.

### D3: `.[]` default query injected only in multi-file mode without explicit `--query`

When `inputs.len() > 1` and no `--query` flag was given, `args.query` is set to `Some(".[]".to_string())` before `App` construction. This keeps the injection in one place (`actual_main`) and avoids coupling `App` to input-count logic.

### D4: Cursor initialisation via `textarea.move_cursor(CursorMove::Jump(0, col))`

`tui_textarea` provides `CursorMove::Jump(row, col)`. After inserting the initial query string we call `move_cursor(CursorMove::Jump(0, col))` where `col` is clamped to `query.chars().count()`. This is entirely in the binary crate (`main.rs` or a small helper in `loop_state.rs` / `run()`).

### D5: Initial suggestion compute — one blocking call before first draw

In `main_loop`, after constructing `LoopState`, if `app.query_input.textarea.lines()[0]` is non-empty, immediately call `compute_suggestions(app, &mut state, lsp_provider)` (the same function used by debounce). This fires suggestions synchronously on the first iteration before any user input, so ghost text and the dropdown are visible from frame 1.

### D6: `source_label` in multi-file mode

When multiple files are given, `source_label` is set to a comma-separated list of filenames (truncated to 60 chars with `…` if needed). When stdin is mixed in, "stdin" is prepended. This appears in the UI header.

### D7: Cursor column — signed integer with negative-index support

`--cursor` is typed as `i32` in `Args`. Positive values are 0-based character offsets from the start of the query. Negative values count from the end: `-1` places the cursor one character before the end (after the last character), `-2` two characters before the end, and so on. This mirrors Python slice semantics and is the natural choice for scripting callers who know the suffix of a query but not its total length.

Resolution: `resolved_col = if col >= 0 { col as usize } else { query_len.saturating_sub((-col) as usize) }`. The resolved value is then clamped to `[0, query_len]`. Values more negative than `-query_len` saturate to 0 (start). Values larger than `query_len` saturate to `query_len` (end). No errors are raised for out-of-range values — clamping is friendlier for scripting callers.

## Risks / Trade-offs

- [Very large merged arrays] Memory doubles versus reading a single file. Mitigation: no change to current limits; document in help text.
- [Stdin + multiple files ordering] Stdin is always read first (before file arguments), so it appears at index 0 in the merged array. Document this in help text.
- [Initial compute before LSP ready] LSP completions will not be available on the very first frame (LSP starts async). The first compute uses only local completions; LSP completions appear on the next debounce cycle as today. No behaviour regression.

## Migration Plan

No migration needed. All changes are additive CLI flags and a positional-argument multiplicity change. Existing single-`file` invocations continue to work because `Vec<PathBuf>` with one element follows the old code path.
