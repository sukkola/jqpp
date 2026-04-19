## ADDED Requirements

### Requirement: src/main.rs is bounded in size
After refactoring, `src/main.rs` SHALL contain no more than 400 lines.

#### Scenario: Line count check
- **WHEN** the refactor is complete and `wc -l src/main.rs` is run
- **THEN** the output is 400 or fewer

### Requirement: Seven new binary-crate modules exist
The following source files SHALL exist as binary-crate modules declared with `mod X;` in `main.rs`.

#### Scenario: terminal.rs exists
- **WHEN** the project is built
- **THEN** `src/terminal.rs` compiles as a module of the binary crate

#### Scenario: output.rs exists
- **WHEN** the project is built
- **THEN** `src/output.rs` compiles as a module of the binary crate

#### Scenario: loop_state.rs exists
- **WHEN** the project is built
- **THEN** `src/loop_state.rs` compiles and exports `LoopState`

#### Scenario: mouse.rs exists
- **WHEN** the project is built
- **THEN** `src/mouse.rs` compiles and exports `ScrollPane` and all scroll helpers

#### Scenario: suggestions.rs exists
- **WHEN** the project is built
- **THEN** `src/suggestions.rs` compiles and exports `compute_suggestions`

#### Scenario: accept.rs exists
- **WHEN** the project is built
- **THEN** `src/accept.rs` compiles and exports suggestion acceptance helpers

#### Scenario: hints.rs exists
- **WHEN** the project is built
- **THEN** `src/hints.rs` compiles and exports the structural hint helpers

#### Scenario: handlers.rs exists
- **WHEN** the project is built
- **THEN** `src/handlers.rs` compiles and exports `handle_query_input_key`, `handle_pane_key`, `handle_side_menu_key`

### Requirement: LoopState struct consolidates event loop locals
`LoopState` in `src/loop_state.rs` SHALL contain all mutable variables that `main_loop` previously held as local state: `suggestion_active`, `cached_pipe_type`, `lsp_completions`, `debounce_pending`, `last_edit_at`, `last_esc_at`, `discard_sgr_mouse_until`, `suppress_scroll_until`, `drop_scroll_backlog_until`, `footer_message`, `compute_handle`, `pending_qp`, `debounce_duration`.

#### Scenario: LoopState fields present
- **WHEN** `LoopState` is instantiated in `main_loop`
- **THEN** it has all the named fields replacing the previous local variables

### Requirement: Module ownership is coherent
Each function SHALL live in exactly one module, and that module SHALL match the function's responsibility.

#### Scenario: Mouse functions are in mouse.rs
- **WHEN** `apply_mouse_scroll_delta` is referenced
- **THEN** it is defined in `src/mouse.rs`

#### Scenario: compute_suggestions is in suggestions.rs
- **WHEN** `compute_suggestions` is referenced
- **THEN** it is defined in `src/suggestions.rs`

#### Scenario: apply_selected_suggestion is in accept.rs
- **WHEN** `apply_selected_suggestion` is referenced
- **THEN** it is defined in `src/accept.rs`

#### Scenario: maybe_activate_structural_hint is in hints.rs
- **WHEN** `maybe_activate_structural_hint` is referenced
- **THEN** it is defined in `src/hints.rs`

#### Scenario: TtyWriter and TerminalGuard are in terminal.rs
- **WHEN** the terminal is set up
- **THEN** `TtyWriter` and `TerminalGuard` are defined in `src/terminal.rs`

### Requirement: No behavior change
All existing tests SHALL pass without modification after the refactor.

#### Scenario: cargo test passes
- **WHEN** `cargo test` is run after the refactor
- **THEN** all tests pass with the same results as before

#### Scenario: cargo clippy passes
- **WHEN** `cargo clippy` is run after the refactor
- **THEN** no new linting errors are introduced

### Requirement: Tests live in the module they test
Each `#[cfg(test)]` block SHALL be co-located with the functions it tests, not consolidated in `main.rs`.

#### Scenario: Mouse tests in mouse.rs
- **WHEN** the mouse scroll tests exist
- **THEN** they are in `src/mouse.rs`'s test block

#### Scenario: Suggestion tests in suggestions.rs
- **WHEN** the should_hold_output and related tests exist
- **THEN** they are in `src/suggestions.rs`'s test block

#### Scenario: SGR strip tests in accept.rs
- **WHEN** the `strip_sgr_mouse_sequences` tests exist
- **THEN** they are in `src/accept.rs`'s test block

### Requirement: Library crate is unchanged
`src/lib.rs` and all files under `src/completions/`, `src/app.rs`, `src/ui.rs`, `src/widgets/`, `src/keymap.rs`, `src/config.rs`, `src/executor.rs` SHALL NOT be modified.

#### Scenario: lib.rs unchanged
- **WHEN** `git diff src/lib.rs src/app.rs src/executor.rs` is run
- **THEN** there are no changes

### Requirement: main_loop is a dispatcher, not an implementer
After extraction of handler functions, `main_loop` SHALL NOT contain nested match arms deeper than three levels for key event handling.

#### Scenario: QueryInput handling delegated
- **WHEN** a key event arrives for `AppState::QueryInput`
- **THEN** `main_loop` calls `handle_query_input_key(app, &mut state, key, keymap)` rather than handling it inline
