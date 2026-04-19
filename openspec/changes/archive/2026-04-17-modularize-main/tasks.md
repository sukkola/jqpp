## 1. Create src/terminal.rs

- [x] 1.1 Create `src/terminal.rs`. Move `TtyWriter` struct + `Write` impl, `TerminalGuard` struct + `create` method + `Drop` impl, `setup_panic_hook`, `get_tty_handle`, `lsp_on_path` from `main.rs`.
- [x] 1.2 Add `mod terminal;` to `main.rs`. Replace all references with `terminal::X` or add `use crate::terminal::*;`.
- [x] 1.3 Run `cargo check` — must pass before moving on.

## 2. Create src/output.rs

- [x] 2.1 Create `src/output.rs`. Move `OutputMode` enum, `output_mode_from_args`, `selected_output`, `copy_text_to_clipboard`, `right_pane_copy_text`, `parse_input_as_json_or_string` from `main.rs`.
- [x] 2.2 Add `mod output;` to `main.rs`. Fix all use sites.
- [x] 2.3 Run `cargo check`.

## 3. Create src/loop_state.rs

- [x] 3.1 Create `src/loop_state.rs`. Define:
  ```rust
  pub type ComputeResult = (anyhow::Result<(Vec<serde_json::Value>, bool)>, Option<String>);

  pub struct LoopState {
      pub suggestion_active: bool,
      pub cached_pipe_type: Option<String>,
      pub lsp_completions: Vec<jqpp::completions::CompletionItem>,
      pub debounce_pending: bool,
      pub last_edit_at: std::time::Instant,
      pub last_esc_at: Option<std::time::Instant>,
      pub discard_sgr_mouse_until: Option<std::time::Instant>,
      pub suppress_scroll_until: Option<std::time::Instant>,
      pub drop_scroll_backlog_until: Option<std::time::Instant>,
      pub footer_message: Option<(String, std::time::Instant)>,
      pub compute_handle: Option<tokio::task::JoinHandle<ComputeResult>>,
      pub pending_qp: String,
      pub debounce_duration: std::time::Duration,
  }

  impl LoopState {
      pub fn new() -> Self { /* fill with current main_loop initial values */ }
  }
  ```
- [x] 3.2 Add `mod loop_state;` to `main.rs`. In `main_loop`, replace local variable declarations with `let mut state = LoopState::new();` and update all references to use `state.X`.
- [x] 3.3 Run `cargo check`.

## 4. Create src/mouse.rs

- [x] 4.1 Create `src/mouse.rs`. Move `ScrollPane` enum and these functions: `row_in_pane`, `mouse_in_left_pane`, `mouse_in_right_pane`, `focus_state_from_click`, `mouse_scroll_pane`, `mouse_scroll_direction`, `can_scroll_in_direction`, `is_scroll_event`, `scroll_input_suppressed`, `should_drop_boundary_scroll_event`, `apply_mouse_scroll_delta`.
- [x] 4.2 Add `mod mouse;` to `main.rs`. Fix all use sites.
- [x] 4.3 Move the `boundary_scroll_events_are_dropped` test to `src/mouse.rs`.
- [x] 4.4 Run `cargo check`.

## 5. Create src/accept.rs

- [x] 5.1 Create `src/accept.rs`. Move these pure text-manipulation functions: `strip_sgr_mouse_sequences`, `sgr_mouse_sequence_len`, `is_field_path_function_call_start`, `starts_context_aware_function_call`, `is_string_param_value_suggestion`, `apply_suggestion_with_suffix`, `apply_selected_suggestion`, `find_unmatched_open_paren`, `commit_current_string_param_input`, `longest_common_prefix`, `is_string_token_delim`, `extend_to_next_token_boundary`, `expand_string_param_prefix_with_tab`, `cursor_col_after_accept`, `completion_items_to_suggestions`.
- [x] 5.2 Add `mod accept;` to `main.rs`. Fix all use sites.
- [x] 5.3 Move SGR and cursor_col tests to `src/accept.rs`.
- [x] 5.4 Run `cargo check`.

## 6. Create src/hints.rs

- [x] 6.1 Create `src/hints.rs`. Move: `maybe_activate_structural_hint`, `dismiss_structural_hint`, `clear_dismissed_hint_if_query_changed`, `open_suggestions_from_structural_hint`.
- [x] 6.2 These functions take `&mut App` and call `compute_suggestions` (which will be in `suggestions.rs`). For now, keep an `use crate::suggestions::compute_suggestions;` import in `hints.rs`.
- [x] 6.3 Add `mod hints;` to `main.rs`. Fix all use sites.
- [x] 6.4 Run `cargo check`.

## 7. Create src/suggestions.rs

- [x] 7.1 Create `src/suggestions.rs`. Move: `compute_suggestions`, `current_query_prefix`, `active_string_param_prefix_query`, `evaluated_string_param_input`, `split_at_last_pipe`, `split_string_param_query_prefix`, `is_inside_string_literal`, `is_inside_double_quoted_string`, `current_token`, `fuzzy_token_fragment`, `should_offer_builtin_fuzzy`, `lsp_pipe_prefix`, `normalize_lsp_insert_text`, `build_lsp_suggestions`, `should_hold_output_during_suggestions`, `has_non_exact_suggestion_for_prefix`, `suggestion_mode_for_query_edit`.
- [x] 7.2 Add `mod suggestions;` to `main.rs`. Fix all use sites (including the `hints.rs` import from step 6.2).
- [x] 7.3 Move `holds_output_for_partial_suggestion_token`, `releases_output_for_committed_parent_segment` and all other suggestion tests to `src/suggestions.rs`.
- [x] 7.4 Run `cargo check`.

## 8. Create src/handlers.rs

- [x] 8.1 Create `src/handlers.rs`. Extract the three key-handling match arms from `main_loop` into:
  - `pub fn handle_query_input_key(app: &mut App, state: &mut LoopState, key: KeyEvent, keymap: &Keymap) -> bool` (returns `true` if loop should `continue`)
  - `pub fn handle_pane_key(app: &mut App, state: &mut LoopState, key: KeyEvent, keymap: &Keymap)`
  - `pub fn handle_side_menu_key(app: &mut App, state: &mut LoopState, key: KeyEvent, keymap: &Keymap)`
- [x] 8.2 In `main_loop`, replace the three large match arm bodies with calls to these functions:
  ```rust
  AppState::QueryInput => {
      if handle_query_input_key(app, &mut state, key, keymap) { continue; }
  }
  AppState::SideMenu => handle_side_menu_key(app, &mut state, key, keymap),
  _ => handle_pane_key(app, &mut state, key, keymap),
  ```
- [x] 8.3 Add `mod handlers;` to `main.rs`. Fix all use sites.
- [x] 8.4 Run `cargo check`.

## 9. Reduce main_loop

- [x] 9.1 After extracting handlers, audit `main_loop` for any remaining inline helper logic that belongs in one of the new modules. Move any found items.
- [x] 9.2 Verify `main_loop` nesting depth: no key-handling branch deeper than 3 levels.
- [x] 9.3 Verify `main.rs` line count: `wc -l src/main.rs` ≤ 400.

## 10. Clean up and verify

- [x] 10.1 Run `cargo test` — all tests must pass.
- [x] 10.2 Run `cargo clippy` — no new warnings.
- [x] 10.3 Remove any `#[allow(dead_code)]` or `#[allow(unused)]` annotations added during extraction (they indicate a function was moved but its call site not updated).
- [x] 10.4 Verify `git diff src/lib.rs src/app.rs src/executor.rs src/completions/ src/ui.rs src/keymap.rs src/config.rs` shows no changes.
- [x] 10.5 Run the keyboard integration tests (`cargo test --test keyboard_tests`) to confirm end-to-end behavior is unchanged.
