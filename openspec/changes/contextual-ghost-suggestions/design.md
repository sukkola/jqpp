## Context

The existing ghost-text path in `query_input.rs`:
- `ghost_text()` fires when `show_suggestions = true`, `suggestions` is non-empty, and `suggestions[0].insert_text` starts with the current query string
- The ghost renders the suffix in `DarkGray`
- Tab accepts `suggestions[0]` (full insert_text replaces query)
- Up/Down navigation opens the full dropdown (suggestion_index moves, show_suggestions stays true)

`cached_pipe_type` tracks the runtime JSON type flowing *into* the current pipe segment (e.g. `"array"`, `"object"`, `"string"`). This is computed in the background after each debounce. The value *at* the full current query path is `app.results` (the live query output).

The query bar widget has two display modes:
1. `show_suggestions = false, suggestion_active = false` — plain text, no ghost
2. `show_suggestions = true` — ghost text (if conditions met) or dropdown (if index > 0 or explicit open)

`suggestion_active` is a flag in `main.rs` set when the user starts typing a completion trigger. Ghost structural hints must coexist with this: if the user is already typing a completion, structural hints are suppressed.

## Goals / Non-Goals

**Goals:**
- Proactively show `[]` ghost text when the query resolves to an `array` value and doesn't already end with `[` or `[]`
- Proactively show `.` ghost text when the query ends with `[]` and the array elements are objects (so `.field` access is the obvious next step)
- Tab accepts the ghost (inserts the structural character(s))
- Up/Down opens the full context dropdown (same as if the user had typed `[` or `[]`)
- Ghost reappears on backspace to the triggering position
- Ghost suppressed when `suggestion_active = true` (user-initiated completion wins)

**Non-Goals:**
- Suggesting `|` (too ambiguous — many next steps are possible)
- Ghost suggestions for non-structural tokens (no alphabet ghost)
- Suggesting `[]` when query already ends with `[` (user already started the bracket — normal completion takes over)
- Suggesting `.` when the query resolves to a scalar

## Decisions

### D1: Where structural hints are computed

Add `next_structural_hint(query_prefix: &str, input: &Value) -> Option<Vec<CompletionItem>>` in `src/completions/json_context.rs`. This function:
1. Evaluates the path up to the cursor using `find_value_at_path` (already exists)
2. Returns `Some(vec![CompletionItem { label: "[]", insert_text: query + "[]", detail: None }])` if the value is an array and query doesn't end with `[`
3. Returns `Some(vec![CompletionItem { label: ".", insert_text: query + ".", detail: None }])` if query ends with `[]` and the first array element is an object
4. Returns `None` otherwise

This is synchronous and cheap (no jaq execution — just JSON tree walk).

### D2: When to show structural hints

In `main.rs`, after `compute_handle` resolves (results arrive) AND `suggestion_active = false`: call `next_structural_hint` with the current query prefix and JSON input. If it returns `Some(items)`, set `app.query_input.suggestions = items` and `app.query_input.show_suggestions = true` without setting `suggestion_active`. This keeps the hint passive — it won't interfere with the next keystroke triggering normal completions.

On every keystroke that changes the query (and re-triggers debounce), clear any active structural hint if `suggestion_active` becomes true or the query no longer qualifies.

### D3: Coexistence with suggestion_active

`suggestion_active` = user initiated completion (typed a trigger char). When this is true, structural hints are suppressed and do not overwrite `app.query_input.suggestions`. When the user dismisses suggestions (Esc), structural hints re-evaluate on the next results cycle.

A new boolean `structural_hint_active: bool` on `App` distinguishes passive structural hints from user-initiated ones so Tab, Up, Down can be correctly handled: Tab on a structural hint inserts it without locking suggestion mode; Up/Down on a structural hint opens the context dropdown (same as typing the first char of the completion).

### D5: Esc dismisses structural hint without modifying query

When `structural_hint_active = true` and user presses Esc, clear `structural_hint_active`, clear `app.query_input.suggestions`, and set `show_suggestions = false`. The query text is unchanged. To prevent the hint from immediately reappearing on the next results cycle for the same query, store `dismissed_hint_query: Option<String>` on `App`; the hint is suppressed for any query string equal to this value. On the next keystroke that changes the query, clear `dismissed_hint_query`.

### D6: Up/Down behaviour on structural hint

When `structural_hint_active = true` and user presses Up or Down, populate suggestions with the full context-appropriate completions (call `compute_suggestions` with the query + hint char as prefix) and set `suggestion_active = true`, transitioning to normal completion mode. This gives the user the same dropdown they'd see after typing `[`.

## Risks / Trade-offs

- [Ghost text flicker] If the query evaluation is slow (large JSON), structural hints arrive late. Existing debounce (80ms) already smooths this. Structural hint computation is synchronous and near-instant since it only walks the JSON tree. → Low risk.
- [Conflict with fuzzy suggestions] Fuzzy suggestions (from the `fuzzy-completion-search` change) could also populate `suggestions`. Structural hints must check `suggestion_active` and not fire while fuzzy results are shown. Same `suggestion_active` guard handles this.
- [False positive hints] For `.items` where items is an array of mixed types (some elements not objects), suggesting `.` after `[]` would be misleading. → Mitigation: only suggest `.` if the first element is an object; otherwise suggest nothing beyond `[]`.
