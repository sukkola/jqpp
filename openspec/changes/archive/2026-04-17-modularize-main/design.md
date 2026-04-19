## Context

`src/main.rs` is a binary crate entry point, not part of `src/lib.rs`. New modules extracted from it are also binary-crate modules: they live in `src/` but are declared with `mod X;` in `main.rs`, not re-exported through `lib.rs`. They cannot be reached by external tests or the library — this is intentional.

The library crate (`jqpp`) already provides `app`, `completions`, `executor`, `keymap`, `ui`, `widgets`, `config`. None of those change.

### Current responsibilities mapped to lines

| Responsibility | Lines | Size |
|---|---|---|
| Terminal setup / teardown | 77–219 | ~140 |
| CLI / startup | 29–441 | ~250 |
| Async runtime + run() | 443–517 | ~75 |
| main_loop (event loop) | 519–1580 | ~1060 |
| Mouse + scroll utilities | 1582–2127 | ~545 |
| Suggestion acceptance | 1646–1862 | ~220 |
| Structural hint lifecycle | 1892–1971 | ~80 |
| Completion pipeline | 2189–2600 | ~410 |
| Tests | 2601–3391 | ~790 |

`main_loop` is the most egregious: 1,060 lines of a single async function with seven levels of nesting.

## Goals / Non-Goals

**Goals**
- `main.rs` ≤ 400 lines after the refactor
- Each new module has a single, nameable responsibility
- Event loop reads as a dispatcher, not as an implementation
- Tests live in the module they test
- No behavior changes — no new features, no renamed public APIs

**Non-Goals**
- Moving any code into `src/lib.rs` or creating new library-crate modules
- Changing function signatures visible to external callers (there are none)
- Reformatting or rewriting logic — move first, improve later
- Adding new abstractions beyond what's needed to untangle coupling

## Decisions

### D1: Seven binary-crate modules, not a handler hierarchy

Simple flat modules are preferable to a `src/handlers/` subdirectory at this stage. The directory structure would add a `mod.rs` indirection without benefit. If the handler functions grow large in the future, a subdirectory is easy to introduce.

### D2: LoopState struct to tame the event handler signatures

`main_loop` holds ~12 local variables that every event handler needs. Instead of threading them all as individual `&mut` parameters (unreadable) or a tuple (no names), a `LoopState` struct gives them names and lets handler functions be called as:

```rust
fn handle_query_input_key(app: &mut App, state: &mut LoopState, key: KeyEvent, keymap: &Keymap)
```

`LoopState` contains:
- `suggestion_active: bool`
- `cached_pipe_type: Option<String>`
- `lsp_completions: Vec<CompletionItem>`
- `debounce_pending: bool`
- `last_edit_at: Instant`
- `last_esc_at: Option<Instant>`
- `discard_sgr_mouse_until: Option<Instant>`
- `suppress_scroll_until: Option<Instant>`
- `drop_scroll_backlog_until: Option<Instant>`
- `footer_message: Option<(String, Instant)>`
- `compute_handle: Option<JoinHandle<ComputeResult>>`
- `pending_qp: String`
- `debounce_duration: Duration`

`LoopState` lives in `src/loop_state.rs`. It is a plain struct with no methods; `main_loop` constructs it, and handler functions receive `&mut LoopState`.

### D3: Handler functions in their own modules, not nested in main_loop

Three handler functions replace the three large match arms in `main_loop`:
- `fn handle_query_input_key(app, state, key, keymap)` → lives in `src/suggestions.rs` or a new `src/handlers.rs` — whichever is clearer
- `fn handle_pane_key(app, state, key, keymap)` → pane scroll dispatch
- `fn handle_side_menu_key(app, state, key, keymap)` → side menu navigation

These are in a single `src/handlers.rs` module since they are tightly related (all key dispatch) and each is ~50–100 lines.

### D4: Module boundaries

| Module | What goes in | Key rule |
|---|---|---|
| `terminal.rs` | Terminal I/O plumbing only | No app/completion logic |
| `output.rs` | CLI output selection and clipboard | No event loop logic |
| `loop_state.rs` | LoopState struct + ComputeResult type alias | No logic, only data |
| `mouse.rs` | All mouse/scroll event logic | No completion or key logic |
| `suggestions.rs` | compute_suggestions + all query analysis helpers | No App mutation beyond what `compute_suggestions` already does |
| `accept.rs` | Suggestion text manipulation (pure functions) | No event loop state access |
| `hints.rs` | Structural hint helpers (`maybe_activate`, `dismiss`, etc.) | Takes `&mut App`, not `&mut LoopState` |
| `handlers.rs` | Key event dispatch functions | Takes `&mut App` and `&mut LoopState` |

### D5: Tests move to their modules

The 790-line test block at the bottom of `main.rs` tests functions from multiple modules. After extraction, each `#[cfg(test)]` block moves to the file that owns the function under test. `main.rs` retains only tests for functions that stay in `main.rs`.

### D6: Incremental extraction order (safe for CI)

Extract in this order to keep the build green after each step:
1. `terminal.rs` — no cross-dependencies
2. `output.rs` — depends only on `app`, `executor`
3. `loop_state.rs` — define struct, use in main_loop locals
4. `mouse.rs` — depends only on `app`
5. `accept.rs` — pure functions, no App mutation
6. `hints.rs` — depends on `app`, `completions`
7. `suggestions.rs` — depends on `completions`, `executor`, `widgets`
8. `handlers.rs` — depends on all of the above; extracted last

## Risks / Trade-offs

- [Build breakage during extraction] Moving a function before updating all call sites breaks the build. Mitigation: move one module at a time, run `cargo check` after each.
- [Circular visibility] Binary crate modules can freely import each other with `use crate::X;`. No visibility issues.
- [LoopState size] A struct with 12 fields is still large. Mitigation: it replaces 12 loose locals — it's a net win even if imperfect.
- [Handler function signatures] `handle_query_input_key` still needs many parameters. Mitigation: the `LoopState` struct reduces these to `(app, state, key, keymap)`.

## Migration Plan

No migration needed — binary-internal refactor with no public API changes.
