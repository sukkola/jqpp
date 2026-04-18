## 1. Project Scaffolding

- [x] 1.1 Create `Cargo.toml` with edition 2024, baseline rustc 1.90.0, and dependencies: `ratatui`, `crossterm`, `tui-textarea`, `jaq-core`, `jaq-std`, `serde_json`, `tokio`, `clap`, `arboard`
- [x] 1.2 Create `src/main.rs` with CLI arg parsing via `clap`: positional `[file]`, `--lsp` flag
- [x] 1.3 Set up `src/app.rs` with `App` struct, `AppState` enum (`QueryInput`, `LeftPane`, `RightPane`, `SideMenu`), and main event loop skeleton
- [x] 1.4 Verify `cargo build` succeeds with empty stubs

## 2. jaq Executor

- [x] 2.1 Create `src/executor.rs`: read input JSON from stdin or file path; store raw bytes and parsed `serde_json::Value`
- [x] 2.2 Implement `execute(query: &str, input: &Value) -> Result<Vec<Value>, String>` using `jaq-core` + `jaq-std`
- [x] 2.3 Format output values as pretty-printed JSON strings (one value per line for multi-output)
- [x] 2.4 Expose input source label and byte size for the status bar (e.g. `"stdin | 2.4 KB"`)
- [x] 2.5 Write unit tests for valid queries, multi-value output, compile errors, and runtime errors
- [x] 2.6 Cap output at 10 000 results to prevent unbounded memory use on large inputs
- [x] 2.7 Truncate display of raw input >64 KB with `[… N KB total, display truncated]` notice
- [x] 2.8 Offload `execute` to `tokio::task::spawn_blocking` to keep TUI responsive during evaluation

## 3. TUI Layout

- [x] 3.1 Create `src/ui.rs`: implement `draw(frame, app)` using ratatui `Layout` — top query bar, two-column body (left JSON pane + right output pane), bottom footer
- [x] 3.2 Render left pane with raw input JSON (scrollable `Paragraph`) and status bar at bottom
- [x] 3.3 Render right output pane with query result (scrollable `Paragraph`, red text on error) and status bar
- [x] 3.4 Render footer bar with keybinding hints: `enter submit · tab/shift+tab nav · ctrl+c/q quit · ctrl+y copy · ctrl+t toggle input · ctrl+s save · ctrl+m menu`
- [x] 3.5 Implement Ctrl+T to toggle query-input bar visibility; panes expand to fill the gap
- [x] 3.6 Implement Tab focus cycling: QueryInput → LeftPane → RightPane → QueryInput
- [x] 3.7 Highlight active pane border with accent color; inactive panes use dim border
- [x] 3.8 Write UI rendering tests: query bar title, input/output pane titles, content rendering, error rendering, pipe query results, large input truncation, footer keybindings

## 4. Query Input Widget

- [x] 4.1 Create `src/widgets/query_input.rs` wrapping `tui-textarea` for single-line editing
- [x] 4.2 Implement query history: push on Enter, Up arrow navigates when dropdown is not active
- [x] 4.3 Implement ghost-text rendering: compute suffix from top suggestion and render as `Color::DarkGray` span after cursor
- [x] 4.4 Implement suggestion dropdown widget rendered below the query bar (left + right + bottom borders, no top border); show all suggestions with highlighted item
- [x] 4.5 Down arrow while dropdown active: move highlight down (wrapping); update ghost text
- [x] 4.6 Up arrow while dropdown active: move highlight up; at first item dismiss dropdown
- [x] 4.7 Esc while dropdown active: dismiss dropdown, clear ghost text, retain typed value
- [x] 4.8 Tab/Enter while dropdown visible: accept full suggestion, position cursor after `("` for string-parameter functions, after `(` for other-parameter functions, at end for no-parameter functions
- [x] 4.9 Tab while no dropdown: pass through to layout focus-cycle
- [x] 4.10 Down arrow while dropdown not visible: open dropdown from cache (or force debounce if cache empty)
- [x] 4.11 Backspace/Delete always sets `suggestion_active = true` so completions re-trigger mid-word
- [x] 4.12 Write unit tests for `suggestion_rect` positioning, ghost text, rendered dropdown (no "Suggestions" heading), cursor-column tracking

## 5. JSON-Context Completion Provider

- [x] 5.1 Create `src/completions/json_context.rs`: accept current query string + input `Value`, return `Vec<CompletionItem>`
- [x] 5.2 Walk query result (or raw input on invalid query) up to depth 4, collect object keys as field-path strings
- [x] 5.3 Filter candidates by the prefix after the last `.` or `|` in the query
- [x] 5.4 Implement 80 ms debounce: only recompute after idle period since last keystroke
- [x] 5.5 Write unit tests: top-level keys, nested paths, depth cap, prefix filtering, invalid-query fallback

## 6. jq Builtins Catalog

- [x] 6.1 Create `src/completions/jq_builtins.rs` with compile-time catalog of ~80 jq builtin functions
- [x] 6.2 Annotate each entry with `InputType`: `Any`, `NonBoolean`, `String`, `Number`, `Array`, `Object`, `StringOrArray`, `ArrayOrObject`
- [x] 6.3 Implement `get_completions(token, input_type) -> Vec<CompletionItem>` with two-pass ordering (type-specific first, then universal)
- [x] 6.4 `length` annotated `NonBoolean` — excluded when pipe input is boolean (avoids `true | length` runtime error)
- [x] 6.5 `@base64` annotated `String` — excluded for non-string input
- [x] 6.6 Parameterized functions include argument placeholder in `insert_text` (e.g. `split(",")`, `ltrimstr("")`)
- [x] 6.7 Implement `jq_type_of(val: &Value) -> &'static str` mapping serde_json Values to jq type strings
- [x] 6.8 Write unit tests: type filtering for string/number/array/object/boolean/null, length exclusions, @base64 exclusions, no duplicates, typed before universal ordering

## 7. LSP Completion Provider

- [x] 7.1 Create `src/completions/lsp.rs`: spawn `jq-lsp` subprocess with stdin/stdout pipes (only when `--lsp` passed)
- [x] 7.2 Implement JSON-RPC framing (Content-Length header) for LSP messages
- [x] 7.3 Send `initialize` + `initialized` handshake; show "LSP initializing…" footer until ready
- [x] 7.4 Implement 5-second handshake timeout: on timeout show "LSP unavailable" footer for 3 s
- [x] 7.5 Background tokio task reads LSP responses from stdout and forwards via `mpsc` channel
- [x] 7.6 On each debounced edit: send `textDocument/didChange` then `textDocument/completion`; parse `CompletionItem` list
- [x] 7.7 Handle `textDocument/publishDiagnostics`: display first error/warning in footer in red
- [x] 7.8 Clear footer diagnostic when diagnostics array is empty
- [x] 7.9 On app exit: send `shutdown` + `exit`; wait for child process to terminate
- [x] 7.10 LSP cache only updated on non-empty responses (prevents jq-lsp returning 0 items for `as` keyword clearing the list)
- [x] 7.11 Client-side stale-cache filtering: filter LSP cache by current token prefix before merging (prevents flicker when LSP response lags behind typing)
- [x] 7.12 Write integration test (with mock LSP server stub) for completion request/response round-trip

## 8. Completion Merging

- [x] 8.1 Define `CompletionItem { label, detail, insert_text }` in `src/completions/mod.rs`
- [x] 8.2 Implement `compute_suggestions(query, json_input, lsp_completions, pipe_type)` as single source of truth for building the suggestion list; called by both the debounce path and the LSP response handler
- [x] 8.3 Merge order: json_context → jq_builtins (type-filtered) → LSP; first-seen deduplication by label
- [x] 8.4 Detect pipe-prefix type via `spawn_blocking`: evaluate expression left of last `|`; store result type in `cached_pipe_type`; pass to `compute_suggestions`
- [x] 8.5 Write tests for compute_suggestions: pipe prefix handling, type-filtered builtins, stale LSP cache filtering, backspace re-triggering, array/number pipe contexts

## 9. Side Menu

- [x] 9.1 Create `src/widgets/side_menu.rs`: static item list `["Query","Config","Runs","Output","History","Saved"]` with selected index
- [x] 9.2 Implement Ctrl+M in app key handler: toggle `side_menu_visible` flag; adjust layout column widths
- [x] 9.3 Render side-menu column when visible: item list with highlight on selected item; empty panel body
- [x] 9.4 When side-menu has focus: Up/Down navigate items (wrapping); no action on Enter

## 10. Clipboard and Save

- [x] 10.1 Implement Ctrl+Y: write current query string to system clipboard via `arboard`; show "copied" in footer for 2 s
- [x] 10.2 Implement Ctrl+S: write output pane content to `jqt-output.json` in CWD; show "saved" in footer for 2 s
- [x] 10.3 Implement `q` quit (when focus not on query input) and `Ctrl+C` quit from any state with clean terminal restore
- [x] 10.4 Add `--version` flag output from `Cargo.toml` version field
- [x] 10.5 Write `README.md` documenting usage, keybindings, `--lsp` flag, and known jaq compatibility gaps
- [x] 10.6 Verify `cargo clippy -- -D warnings` passes with no errors
- [x] 10.7 Verify `cargo test` passes for all unit and integration tests (134 tests as of latest implementation)

## 11. Format Operators (@csv / @tsv)

- [x] 11.1 Implement `strip_format_op(query) -> Option<(String, &'static str)>` in `executor.rs`: detect trailing `| @csv` / `| @tsv`
- [x] 11.2 Implement `format_csv(v)` and `format_tsv(v)` Rust formatters with correct quoting/escaping per RFC 4180
- [x] 11.3 Implement `execute_query(query, input) -> Result<(Vec<Value>, bool)>`: wraps `execute` + format-op interception; returns `raw_output` flag
- [x] 11.4 Replace all `execute` call sites in `main.rs` with `execute_query`; propagate `app.raw_output`
- [x] 11.5 Update `format_results(results, raw)` to skip JSON quotes when `raw = true`
- [x] 11.6 Fix `Ctrl+Y` in `RightPane` to copy `app.error` text when an error is shown, and raw output when `raw_output` is set
- [x] 11.7 Add `@csv`, `@tsv`, `@base64`, `@base64d`, `@uri`, `@html`, `@sh` to jq_builtins catalog with correct `InputType` annotations

## 12. Suggestion Dropdown Improvements

- [x] 12.1 Add `pub const DROPDOWN_VISIBLE: usize = 11` to `query_input.rs`
- [x] 12.2 Add `suggestion_scroll: usize` field to `QueryInput`
- [x] 12.3 Implement `clamp_scroll()`: adjusts `suggestion_scroll` so `suggestion_index` is always in `[scroll, scroll + DROPDOWN_VISIBLE)`
- [x] 12.4 Update dropdown render in `ui.rs` to slice `suggestions[scroll..scroll+visible]` and use relative highlight index
- [x] 12.5 Remove ratatui `ListState` from dropdown rendering (replaced by explicit scroll)
- [x] 12.6 Add `@` to trigger characters in the key handler
- [x] 12.7 Write 6 rolling-window unit tests in `query_input.rs`

## 13. Paste Performance

- [x] 13.1 Add `EnableBracketedPaste` to terminal setup in `TerminalGuard::new`
- [x] 13.2 Add `DisableBracketedPaste` to `TerminalGuard::drop`, `on_exit_signal`, and panic hook
- [x] 13.3 Handle `Event::Paste(text)` in main loop: insert all chars directly into textarea, suppress per-character suggestion pipeline, fire single debounce

## 14. Double-Esc to Clear Query

- [x] 14.1 Add `last_esc_at: Option<Instant>` to event-loop state
- [x] 14.2 First Esc: dismiss dropdown (existing), arm `last_esc_at`
- [x] 14.3 Second Esc within 500 ms: clear textarea content, fire immediate debounce

## 15. Non-Blocking Query Execution

- [x] 15.1 Add `compute_handle: Option<JoinHandle<ComputeResult>>` and `pending_qp: String` to event-loop state
- [x] 15.2 Move `spawn_blocking` call to spawn-without-await; store handle in `compute_handle`
- [x] 15.3 Add poll section at top of main loop: `handle.is_finished()` check; instant `.await` when ready; apply results and refresh suggestions
- [x] 15.4 No-executor branch remains synchronous (no I/O)

## 16. Mid-Query Completions

- [x] 16.1 Extract `cursor_col = textarea.cursor().1` and `query_prefix = query.chars().take(cursor_col).collect()` in debounce block
- [x] 16.2 Pass `query_prefix` (not full query) to `compute_suggestions` and as `q` for pipe-type detection
- [x] 16.3 Update LSP handler to extract `query_prefix` from cursor position before calling `compute_suggestions`
- [x] 16.4 Accept-suggestion (Enter and Tab): capture `suffix = full.chars().skip(cursor_col).collect()`; set new textarea text to `format!("{}{}", insert_text, suffix)`

## 17. Builtin Catalog Expansion

- [x] 17.1 Add `reduce`, `foreach`, `until`, `while` with template insert_text as `InputType::Any`
- [x] 17.2 Update `any`/`all` insert_text to predicate form: `any(. > 0)` / `all(. > 0)`
- [x] 17.3 Change `.field` placeholder to `.key` in `sort_by`, `group_by`, `unique_by`, `min_by`, `max_by`
- [x] 17.4 Add `recurse_down`, `limit`, `first(expr)`, `last(expr)`, `range` as `InputType::Any`
- [x] 17.5 Remove duplicate string-only entries (`ascii_upcase`, `ascii_downcase`, `ltrimstr`, `rtrimstr`, `split`, `indices`) from `Any` section — keep only their typed `String`/`StringOrArray` forms
- [x] 17.6 Write additional type-exclusion tests confirming string-only functions absent for number/array/boolean input

## 18. Complex Query Tests

- [x] 18.1 Add 16 executor tests covering `fromjson`, `reduce`, `group_by`, `unique_by`, `sort_by | reverse`, `variable binding ($x)`, `try-catch`, `select`, multi-pipe, `@base64` round-trip, `@csv` output
