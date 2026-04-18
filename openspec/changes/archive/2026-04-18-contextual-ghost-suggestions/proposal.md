## Why

Building a jq query requires knowing which structural characters to type next (`[]` for arrays, `.` after array iteration, `|` to pipe). Users — especially those unfamiliar with jq syntax — have to remember these characters and find them on the keyboard. jqpp already knows the type of the value at the current query position (from the live JSON input), so it can proactively suggest the obvious structural next step as inline ghost text, removing the need to type the trigger character at all.

## What Changes

- After each debounce cycle, inspect the JSON value at the resolved current query path
- If the next obvious structural step can be inferred from that type, show it as inline gray ghost text in the query bar (the existing ghost-text rendering path is already in place)
- Ghost suggestions are **structural only**: `[]` (array iteration), `.` (object descent after array), `|` is not auto-suggested (too ambiguous). Alphabetic text is never ghost-suggested by this feature
- The ghost text is selectable with Tab (accept full suggestion) or Up/Down (open the suggestion dropdown showing all options for that context, e.g. `[]`, `[0]`, `[1]` …)
- Ghost text updates correctly when backspacing: if the user erases back to a position where the context is again an array, the ghost reappears
- No ghost is shown when the query is empty, when the result is a scalar, when suggestions are already active from user typing, or when the current position already ends with the relevant structural character

## Capabilities

### New Capabilities

- `proactive-structural-hints`: Automatic ghost-text suggestion of the structurally obvious next step based on the live JSON type at the current query path, without requiring the user to type a trigger character

### Modified Capabilities

(none — the ghost text rendering path in `query_input.rs` and `show_suggestions` logic in `main.rs` are extended, not replaced)

## Impact

- `src/completions/json_context.rs`: new `next_structural_hint(query: &str, input: &Value) -> Option<Vec<CompletionItem>>` that infers structural suggestions from value type at path
- `src/main.rs`: after compute results arrive, call `next_structural_hint` and populate `app.query_input.suggestions` + enable ghost text when no user-initiated suggestion is active
- `src/widgets/query_input.rs`: no structural changes needed — existing `ghost_text()` and `show_suggestions` path already supports this
- `README.md`: brief mention in the completions/intellisense section
