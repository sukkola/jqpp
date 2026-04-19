## Why

`src/main.rs` has grown to 3,400 lines and carries six distinct responsibilities in a single flat file: terminal lifecycle, CLI argument handling, the async event loop, completion pipeline assembly, suggestion acceptance logic, mouse/scroll handling, and structural hint state management. Every new feature lands in `main.rs` by default, making it increasingly hard to navigate, review, and test. There are no internal module boundaries preventing cross-concern coupling, and the `#[cfg(test)]` block at the bottom tests functions that conceptually belong to several different domains.

## What Changes

- `main.rs` is reduced to its irreducible core: CLI parsing (`Args`, `main`, `actual_main`), the Tokio runtime setup (`run`), and the event loop dispatcher (`main_loop`). All helper logic moves out.
- Six new binary-crate modules are introduced alongside `main.rs`:
  - `src/terminal.rs` — `TtyWriter`, `TerminalGuard`, `setup_panic_hook`, `get_tty_handle`, `lsp_on_path`
  - `src/output.rs` — `OutputMode`, `output_mode_from_args`, `selected_output`, `copy_text_to_clipboard`, `right_pane_copy_text`, `parse_input_as_json_or_string`
  - `src/loop_state.rs` — `LoopState` struct bundling the mutable variables that `main_loop` currently holds as locals; shared across handler calls
  - `src/mouse.rs` — `ScrollPane`, all mouse hit-testing, scroll suppression, boundary-drop, delta application
  - `src/suggestions.rs` — `compute_suggestions`, query analysis helpers (`current_token`, `is_inside_string_literal`, `split_at_last_pipe`, etc.), LSP suggestion patching, debounce compute dispatch
  - `src/accept.rs` — suggestion acceptance: `apply_selected_suggestion`, `expand_string_param_prefix_with_tab`, `commit_current_string_param_input`, text-manipulation helpers, `cursor_col_after_accept`
  - `src/hints.rs` — structural hint lifecycle: `maybe_activate_structural_hint`, `dismiss_structural_hint`, `clear_dismissed_hint_if_query_changed`, `open_suggestions_from_structural_hint`
- The event handler blocks inside `main_loop` (QueryInput, SideMenu, pane keys) are extracted to standalone functions that accept `(&mut App, &mut LoopState)` — removing the deeply-nested match arms from the loop body
- Tests move to the module they actually test; `main.rs` keeps only tests for logic that stays there

## Capabilities

### New Capabilities

*(none — this is a pure internal refactor)*

### Modified Capabilities

*(none — no observable behavior changes)*

## Impact

- `src/main.rs` — reduced to ~400 lines (CLI + run + main_loop dispatcher)
- New files: `src/terminal.rs`, `src/output.rs`, `src/loop_state.rs`, `src/mouse.rs`, `src/suggestions.rs`, `src/accept.rs`, `src/hints.rs`
- All new files are binary-crate modules (declared as `mod X;` in `main.rs`), not library-crate changes
- No changes to `src/lib.rs` or any file under `src/completions/`, `src/app.rs`, `src/ui.rs`, `src/widgets/`, `src/keymap.rs`, `src/config.rs`, `src/executor.rs`
- Existing integration tests (`tests/keyboard_tests.rs`) and the library test suites are unaffected
