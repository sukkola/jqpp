## 1. Args struct: new flags and multi-file positional

- [x] 1.1 In `src/main.rs`, change `file: Option<PathBuf>` to `files: Vec<PathBuf>` with `#[arg(num_args(0..))]`. Update every reference in `actual_main` and `run` that currently reads `args.file`.
- [x] 1.2 Add `/// Initial query string` `#[arg(long)] query: Option<String>` to `Args`.
- [x] 1.3 Add `/// Initial cursor column (0-based from start; negative counts from end)` `#[arg(long, allow_hyphen_values = true)] cursor: Option<i32>` to `Args`. The `allow_hyphen_values` attribute is required so clap does not treat negative numbers as unknown flags.
- [x] 1.4 Run `cargo check` — must pass before continuing.

## 2. Multi-file input loading

- [x] 2.1 In `actual_main`, replace the single-file read block with a function `load_inputs(files: &[PathBuf], stdin_is_terminal: bool) -> Result<(Vec<u8>, Vec<String>)>` (raw bytes merged, source labels). Return empty bytes + empty labels if no inputs.
- [x] 2.2 Implement the merge: read stdin (if piped) as the first input, then each file in order. Parse each chunk individually with `parse_input_as_json_or_string`. If more than one input value exists, wrap them in `serde_json::Value::Array(values)` and serialise back to bytes for `raw_input`. Single input stays unwrapped.
- [x] 2.3 Build `source_label`: join individual labels with `", "`, truncate to 60 chars with `…` if needed. Label for stdin is `"stdin"`, label for each file is its filename (not full path unless the filename alone is ambiguous).
- [x] 2.4 When more than one input source is present and `args.query` is `None`, set `args.query = Some(".[]".to_string())` before constructing `App`.
- [x] 2.5 Run `cargo check`.

## 3. Initial query and cursor application

- [x] 3.1 In `run()`, after `App::new()` and `app.executor = executor;`, if `args.query` is `Some(q)` and `q` is non-empty: call `app.query_input.textarea.insert_str(&q)` to set the query bar content.
- [x] 3.2 After inserting the query, resolve and apply cursor: if `args.cursor` is `Some(col)`, compute `resolved = if col >= 0 { col as usize } else { query_len.saturating_sub((-col) as usize) }`, then clamp to `[0, query_len]`. Call `app.query_input.textarea.move_cursor(tui_textarea::CursorMove::Jump(0, resolved))`.
- [x] 3.3 If `args.query` is `None` or empty, do nothing (cursor stays at position 0).
- [x] 3.4 Run `cargo check`.

## 4. Initial suggestion compute

- [x] 4.1 In `main_loop`, after the existing `Executor::execute(".", &input)` initial compute block and before the `while app.running` loop, add: if the query bar is non-empty (`!app.query_input.textarea.lines()[0].is_empty()`), call `crate::suggestions::compute_suggestions(app, &mut state, lsp_provider.as_mut())` to fire suggestions before the first draw.
- [x] 4.2 Verify that `compute_suggestions` is `pub` and accessible from `main_loop` (it should already be, since it's in `crate::suggestions`).
- [x] 4.3 Run `cargo check`.

## 5. Headless mode: apply --query for --print-* flags

- [x] 5.1 In `run()`, inside the `JQPP_SKIP_TTY_CHECK` headless branch, when `output_mode.is_some()`: replace the hardcoded `"."` query with the actual query from `args.query.as_deref().unwrap_or(".")` when calling `Executor::execute`.
- [x] 5.2 Run `cargo check`.

## 6. Unit tests — multi-file input logic

- [x] 6.1 Add unit tests in `src/main.rs` (or a new `src/input.rs` if `load_inputs` is extracted): `single_file_not_wrapped_in_array`, `two_files_merged_into_array`, `three_files_merged_into_array`, `non_json_file_becomes_string_in_array`.
- [x] 6.2 Add `mixed_types_preserved_in_merged_array`: files containing a JSON object, a JSON array, and a JSON string are merged correctly.
- [x] 6.3 Add `duplicate_file_produces_two_entries`: same file path given twice → two copies in the array.
- [x] 6.4 Add `source_label_for_two_files`: label is `"a.json, b.json"`.
- [x] 6.5 Add `source_label_truncated_when_long`: label longer than 60 chars is truncated with `…`.
- [x] 6.6 Add `default_query_is_dot_slice_for_two_inputs`: when two files and no `--query`, the returned default query is `.[]`.
- [x] 6.7 Add `explicit_query_overrides_dot_slice_default`: when two files and `--query '.[] | .name'`, the default `.[]` is NOT injected.
- [x] 6.8 Run `cargo test` — all new unit tests must pass.

## 7. Unit tests — cursor resolution

- [x] 7.1 Add `cursor_positive_within_bounds_unchanged`: `resolve_cursor(2, 5)` → 2.
- [x] 7.2 Add `cursor_positive_clamped_to_query_length`: `resolve_cursor(999, 3)` → 3.
- [x] 7.3 Add `cursor_positive_zero_for_empty_query`: `resolve_cursor(0, 0)` → 0.
- [x] 7.4 Add `cursor_negative_minus_one_is_end`: `resolve_cursor(-1, 4)` → 3.
- [x] 7.5 Add `cursor_negative_minus_query_len_is_start`: `resolve_cursor(-4, 4)` → 0.
- [x] 7.6 Add `cursor_negative_more_than_query_len_clamps_to_zero`: `resolve_cursor(-999, 3)` → 0.
- [x] 7.7 Add `cursor_negative_mid_query`: `resolve_cursor(-3, 10)` → 7 (10 - 3).
- [x] 7.8 Run `cargo test`.

## 8. Integration tests — initial query flag

- [x] 8.1 In `tests/pipe_integration.rs` (or a new `tests/cli_flags.rs`), add test `query_flag_sets_initial_query_in_headless_mode`: launch with `--query '.name' --print-query` in headless mode (`JQPP_SKIP_TTY_CHECK`), assert stdout is `.name\n`.
- [x] 8.2 Add `query_flag_with_print_output_evaluates_query`: launch with a simple JSON input, `--query '.name'`, and `--print-output` in headless mode; assert stdout is the `name` field value.
- [x] 8.3 Add `empty_query_flag_produces_empty_print_query`: `--query '' --print-query` → stdout is `\n`.
- [x] 8.4 Add `no_query_flag_leaves_bar_empty`: `--print-query` without `--query` → stdout is `\n`.
- [x] 8.5 Add `negative_cursor_resolves_from_end`: `--query 'sort_by(.price)' --cursor -7 --print-query` in headless mode exits without error (validates clap accepts negative values without treating them as flags).
- [x] 8.6 Run `cargo test --test cli_flags` (or equivalent).

## 9. Integration tests — multi-file input

- [x] 9.1 Add test `two_files_produce_merged_array`: pass two temp files with known JSON objects; `--print-input` in headless mode should produce a two-element JSON array on stdout.
- [x] 9.2 Add `two_files_default_query_is_dot_slice`: two files + `--print-query` → stdout is `.[]`.
- [x] 9.3 Add `two_files_explicit_query_overrides_default`: two files + `--query '.[] | .id' --print-query` → stdout is `.[] | .id`.
- [x] 9.4 Add `missing_file_in_multi_file_list_exits_nonzero`: one valid file + one nonexistent path → process exits with non-zero status before TUI opens.
- [x] 9.5 Add `single_file_not_affected`: one file + `--print-input` → original JSON (not wrapped in array).
- [x] 9.6 Run `cargo test`.

## 10. Clean up and verify

- [x] 10.1 Run `cargo test` — all tests must pass.
- [x] 10.2 Run `cargo clippy` — no new warnings.
- [x] 10.3 Run `cargo check` on the full workspace.
- [x] 10.4 Verify `git diff src/lib.rs src/app.rs src/executor.rs src/completions/ src/ui.rs src/keymap.rs src/config.rs` shows no changes (library crate unchanged).
- [x] 10.5 Manually smoke-test: `echo '{"a":1}' | cargo run -- --query '.a' --cursor 2` — tool should open with `.a` in query bar and cursor after `.`.
- [x] 10.6 Manually smoke-test multi-file: `cargo run -- examples/users.json examples/orders.json` — tool should open with `.[]` in query bar and a two-element array in the input pane.
