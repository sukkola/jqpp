## Context

`jqt` is a new Rust TUI tool inspired by the Go `jqp` project. It targets developers who frequently inspect and transform JSON in the terminal and want intelligent query assistance beyond what a bare `jq` REPL provides. The existing Go reference implementation (`/Users/sampo/Sources/photo-evaluator/jqadditions/jqp`) demonstrates the ghost-text input approach and LSP plumbing; `jqt` adopts those patterns in Rust using `ratatui` + `crossterm` as the rendering backend and `jaq` as the jq engine.

Key constraints:
- No external `jq` binary required (jaq is compiled in)
- `jq-lsp` is an optional runtime dependency (enabled with `--lsp` flag)
- Must be keyboard-navigable; no mouse required
- Layout must visually match jqp's two-pane design with an optional third side-menu column

## Goals / Non-Goals

**Goals:**
- Rust binary `jqt` with jqp-compatible UX (same pane layout, same keybindings)
- Ghost-text completions from two independent, merged providers
- JSON-context provider: parse live query output to surface field names/values
- LSP provider: optional jq-lsp subprocess with JSON-RPC completions + diagnostics
- Side-menu column (stub items; hidden by default, toggled by keybinding)
- History (up/down arrow cycles through previous queries)
- Ctrl+Y copy, Ctrl+T toggle input, Ctrl+S save output

**Non-Goals:**
- Mouse support in v1
- Saving/loading named query files from disk (History pane is in-session only for v1)
- Remote/networked LSP (stdio subprocess only)
- Syntax highlighting of jq queries in the input field

## Decisions

### D1: ratatui + crossterm for TUI

**Choice**: `ratatui` with `crossterm` backend.

**Rationale**: ratatui is the de-facto standard Rust TUI library (successor to tui-rs), has excellent documentation, and handles full-screen layouts, event loops, and block/paragraph widgets out of the box. `crossterm` is cross-platform (Linux, macOS, Windows).

**Alternatives considered**: `cursive` (higher-level but less flexible for custom widget composition), raw termion/crossterm without ratatui (too low-level for the layout complexity needed).

---

### D2: jaq as jq engine (library crate)

**Choice**: Depend on `jaq-core` + `jaq-std` crates directly.

**Rationale**: jaq is a Rust-native jq implementation with a public library API. Linking it as a library avoids subprocess spawning, eliminates the need for an installed `jq` binary, and enables synchronous or async query evaluation within the same process.

**Alternatives considered**: Spawning external `jq` binary (portable but adds runtime dependency and IPC overhead), `jq-rs` crate (wraps C libjq, requires libjq at link time).

---

### D3: Three-source completion merging strategy

**Choice**: Three independent providers produce `Vec<CompletionItem>` independently; results are merged with first-seen deduplication in priority order: (1) json_context, (2) jq_builtins (type-filtered), (3) LSP stale cache (client-side filtered by token prefix).

**Rationale**: json_context gives the most contextually relevant completions (field paths). A compile-time builtin catalog (`jq_builtins.rs`) provides type-aware function suggestions without requiring an active LSP — it filters by the runtime type produced by the expression to the left of `|`. LSP remains as a third-tier fallback for custom functions and diagnostics. Single `compute_suggestions()` function is the sole construction point; both the debounce tick and the LSP response handler call it to prevent divergence (the root cause of the original flicker bug).

---

### D4: Ghost-text rendering via ratatui `Paragraph` with styled spans

**Choice**: Render the query input as a `Paragraph` with two spans: (1) the typed text, (2) the ghost suffix in a dimmed/gray style. A `TextArea` widget from `tui-textarea` crate handles editing; the ghost suffix is overlaid as a separate styled span.

**Rationale**: `tui-textarea` provides a battle-tested single-line text editor with cursor tracking, undo, and clipboard. Overlaying the ghost text as a styled suffix keeps the implementation simple — Tab or Right-arrow accepts the full suggestion.

**Alternatives considered**: Implementing a custom text input widget from scratch (too much effort), using ratatui's built-in `Paragraph` only (no cursor management).

---

### D5: LSP subprocess lifecycle

**Choice**: Spawn `jq-lsp` as a child process with `stdin`/`stdout` pipes at startup (if `--lsp` flag is given). Send `initialize` → `initialized` handshake. For each query edit, send `textDocument/didChange` followed by `textDocument/completion`. Read responses in a background `tokio` task that forwards results via a `tokio::sync::mpsc` channel to the main event loop.

**Rationale**: Matches the pattern proven in the Go reference implementation. Using a background task + channel keeps the TUI event loop non-blocking. `tokio` is needed for async I/O with the LSP subprocess.

**Alternatives considered**: Synchronous blocking read (stalls the TUI), spawning a thread per request (overly complex).

---

### D6: Non-blocking query execution via stored JoinHandle

**Choice**: `tokio::task::spawn_blocking` is called on debounce but the resulting `JoinHandle` is stored in `compute_handle` rather than `.await`-ed inline. At the top of every event loop frame, `handle.is_finished()` is polled (non-blocking); when `true`, `.await` completes instantly and results are applied. A new debounce drops the previous handle (the old task keeps running in the threadpool but its result is discarded) and spawns a fresh computation with the latest query.

**Rationale**: The original inline `.await` blocked the entire main loop for the duration of jaq evaluation. On large datasets with expensive queries (e.g. `fromjson` on 360 KB) this caused the UI to freeze for multiple seconds and made Ctrl+C unresponsive (raw mode disables ISIG, so Ctrl+C is a key event that can't be processed while the loop is blocked). The JoinHandle pattern keeps the event loop at ≤20 ms latency regardless of query cost.

**Alternatives considered**: `tokio::time::timeout` wrapping the await (simpler but repeated short timeouts would spawn many concurrent tasks), a dedicated `mpsc` channel from the blocking thread (adds indirection; JoinHandle polling is sufficient).

---

### D7: Application state machine

**Choice**: Enum `AppState { QueryInput, LeftPane, RightPane, SideMenu }` drives focus and keybinding routing. Each pane is a separate struct with its own `handle_key` method. The top-level `App::handle_key` dispatches based on current state.

**Rationale**: Mirrors jqp's focus-cycling model (Tab cycles panes). Clean separation of pane logic.

### D8: @csv / @tsv via pre-execution interception

**Choice**: Before passing a query to jaq, `strip_format_op` checks whether it ends with `| @csv` or `| @tsv`. If so, the base query (everything before the format operator) is evaluated by jaq and the Rust formatter (`format_csv` / `format_tsv`) is applied to the results. The `raw_output` flag is set so string results display without surrounding JSON quotes.

**Rationale**: `jaq` does not implement `@csv` / `@tsv`. Intercepting at the executor boundary is transparent to all other layers — completions, error handling, and copy/paste all work normally. Alternative of teaching jaq the operators would require patching an upstream crate.

---

### D9: Bracketed paste mode for paste performance

**Choice**: `EnableBracketedPaste` is issued during terminal setup. The event loop handles `Event::Paste(String)` by inserting all characters at once and firing a single debounce, suppressing the per-character suggestion pipeline.

**Rationale**: Without bracketed paste, pasting 200 characters triggers 200 individual `KeyCode::Char` events, each running the full suggestion pipeline (json_context walk + builtin catalog filter + LSP dispatch). This caused multi-second hangs. Bracketed paste collapses the entire paste into one event.

---

### D10: Rolling-window suggestion scroll

**Choice**: A `suggestion_scroll: usize` field is maintained on `QueryInput`. After each `suggestion_index` change, `clamp_scroll()` adjusts the offset so `suggestion_index` is always within `[scroll, scroll + DROPDOWN_VISIBLE)`. The dropdown renders the slice `suggestions[scroll..scroll+visible]` with a relative highlight index.

**Rationale**: The previous approach relied on ratatui's `ListState` scroll recalculation. Because a fresh `ListState` with `offset=0` was constructed every frame, the scroll offset was lost between frames, making it impossible to navigate past the initially visible items. Owning the offset explicitly solves this without any ratatui-internal state.

---

### D11: Cursor-aware completions for mid-query editing

**Choice**: Before calling `compute_suggestions`, the text is sliced at the cursor column: `query.chars().take(cursor_col).collect()`. This prefix is used for token extraction, pipe-prefix type detection, and LSP dispatch. When a suggestion is accepted, any text to the right of the cursor is appended after the `insert_text` so it is preserved.

**Rationale**: Using the full query string when the cursor is in the middle produced wrong completions (the `rfind('|')` found pipes after the cursor, giving the wrong token context) and lost the remainder of the query on accept. Slicing at cursor makes suggestions context-correct at any editing position.

---

## Risks / Trade-offs

- **jaq compatibility gaps** → Some `jq` programs use features not yet in jaq (e.g. `$ENV`, `path()` expressions). `@csv`/`@tsv` are handled by interception (D8). Remaining gaps emit a clear error in the output pane and are documented in the README.
- **LSP startup latency** → `jq-lsp` may take ~100–300 ms to start. A "LSP initializing…" footer indicator is shown; completions fall back to json_context + jq_builtins during this window.
- **tui-textarea cursor state and ghost text overlap** → Ghost text is shown only when the cursor is at the end of the line and `insert_text` extends beyond it. Mid-query editing suppresses ghost text; the dropdown still works at any cursor position (D11).
- **Large JSON inputs** → Mitigated by capping json_context traversal depth at 4 levels and rendering at most 64 KB of raw input. Long-running queries are non-blocking via the JoinHandle pattern (D6).
- **Completion flicker** → A single `compute_suggestions()` function called from both the debounce path and the LSP handler, sharing `cached_pipe_type`, prevents the dropdown from flickering between type-filtered and unfiltered lists.
- **Type suggestion bugs** → `length` on a boolean and `@base64` on a non-string are jq runtime errors. Mitigated by `InputType::NonBoolean` and `InputType::String` annotations; string-only functions are not duplicated in the `Any` section of the catalog.
- **Abandoned in-flight compute tasks** → When a new query supersedes an in-flight `spawn_blocking`, the old JoinHandle is dropped but the OS thread runs to completion. At most one stale thread is live at a time; acceptable given tokio's blocking thread pool size.
- **Windows support** → crossterm is cross-platform but LSP subprocess pipes and bracketed paste behavior may differ. Not a primary target; macOS/Linux are the supported platforms.

## Migration Plan

This is a new binary with no existing users. No migration required. The tool is installed alongside (not replacing) any existing `jq` or `jqp` installation.

## Open Questions (Resolved)

- File argument: implemented — `jqt file.json` reads from file, otherwise stdin.
- Side-menu: hidden by default; `Ctrl+M` toggles; arrow keys navigate items; no action on Enter (stub).
- Rust edition: **2024**, baseline rustc **1.90.0**.
- `@csv`/`@tsv`: implemented via pre-execution interception (D8); `jaq` is not patched.
- Paste performance: resolved via bracketed paste mode (D9).
- Mid-query completions: resolved via cursor-position slicing (D11).
