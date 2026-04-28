# Main Event Loop

`src/main.rs: main_loop()` is the core async function that drives the TUI. It runs once per session and owns `LoopState`.

## Startup sequence (before the loop)

1. **Startup keys** — if `--query` contained `\n` or `\t` control chars, they are extracted into `app.startup_keys` at parse time and injected as synthetic key events before the first real terminal read. This lets the caller pre-execute Enter/Tab presses as part of the initial state.

2. **Initial jq evaluation** — the preloaded query (if any) is run synchronously via `spawn_blocking` so the output pane is populated on the first frame.

3. **Initial suggestions** — if the query is non-empty, `compute_suggestions` runs immediately and `show_suggestions` / `suggestion_active` are set. This is what makes `--query` mode start with the suggestion box open when the context warrants it.

4. **Footer message** — any startup message stored in `app.footer_message` is moved into `state.footer_message` for TTL management.

## Per-iteration steps (in order)

```
while app.running {
    handle_finished_computes   // drain async jq results, update suggestions
    terminal.draw              // render one frame
    handle_lsp_message (loop)  // drain LSP channel (non-blocking)
    poll_and_process_events    // read ≤N key/mouse events (8 ms poll)
    run_debounced_compute      // fire deferred work after idle period
    footer_message TTL check   // expire stale status messages
}
```

### `handle_finished_computes`

Drains the async jq executor channel. On each finished result it:
- Updates `app.results` (output pane content).
- Recomputes suggestions if `suggestion_active` is set, or activates the contains-builder context automatically if the cursor is inside a `contains(` call.
- Falls back to structural hints if suggestions are empty.

### `terminal.draw`

Renders the full frame through `ui::draw`. The query bar widget uses ghost-text overlay: when the first suggestion's `insert_text` starts with the current query, the remaining suffix is rendered in DarkGray inline. **This ghost text is visible to `agent-tui screenshot --strip-ansi` and will be read by `tui_read_query`.**

### LSP drain

Non-blocking `try_recv` loop processes any completions that arrived since the last frame. Does not block the render cycle.

### `poll_and_process_events`

Calls `event::poll` with an 8 ms timeout. Processes up to `MAX_READ_EVENTS` key/mouse events per iteration via `handle_query_input_key` and friends. `pending_startup_events` are drained first before real terminal reads.

### `run_debounced_compute`

After the user stops typing for `debounce_duration` (80 ms), triggers the async jq execution and LSP completion fetch. The debounce timer resets on every keystroke via `state.debounce_pending`.

## Key state flags

| Flag | Where set | Meaning |
|---|---|---|
| `suggestion_active` | handlers, debounce | Whether the suggestion system should compute/show suggestions on next debounce |
| `debounce_pending` | handlers | A keystroke arrived; fire deferred compute after idle |
| `show_suggestions` | suggestions | Whether the dropdown is rendered this frame |
| `structural_hint_active` | hints | A one-shot structural hint (e.g. `[]`) is showing instead of a full suggestion list |

## Ghost text and `tui_read_query`

`QueryInput::ghost_text()` returns the suffix of `suggestions[suggestion_index].insert_text` beyond the current textarea content when that insert_text starts with the current query. This suffix is rendered in DarkGray after the cursor.

Because `agent-tui screenshot --strip-ansi` strips ANSI colour codes but preserves all characters, the ghost text appears in the raw screenshot alongside the real query text. `tui_read_query` cannot distinguish them — it reads both as a single line.

**Consequence for TUI tests:** `query_contains` assertions will include ghost-text content. After accepting a suggestion that leaves the cursor mid-builder (e.g. after Tab on a contains object value), the next suggestion's label will appear appended to the visible query. Assertions should account for this, or use `suggestions_contain` / `suggestions_not_contain` instead of `query_contains` when the ghost text is not predictable.

## Teardown

After `app.running` goes false (user pressed Ctrl-C / Esc-to-quit), the LSP provider is shut down gracefully.
