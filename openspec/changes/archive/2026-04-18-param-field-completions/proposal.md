## Why

Several jq built-in functions accept a field-path expression (e.g. `.name`, `.price`) as their parameter — functions like `sort_by`, `group_by`, `unique_by`, `min_by`, `max_by`, `del`, and `path`. When a user types `sort_by(.` or `del(.na`, the intellisense is silent even though the available field names can be inferred directly from the live JSON input. This creates a gap: field completions work everywhere else (bare paths, object constructors) but go dark inside these function argument slots.

## What Changes

- Detect when the cursor is inside a function-parameter parenthesis for a recognised "field-path function"
- Resolve the appropriate JSON context for that function:
  - For array-element functions (`sort_by`, `group_by`, `unique_by`, `min_by`, `max_by`): use the first element of the array that feeds into the function
  - For input-path functions (`del`, `path`): use the current input value directly
- Feed the detected inner prefix (e.g. `.na` from `sort_by(.na`) into the existing `json_context::dot_path_completions` logic
- Return completions whose `insert_text` is the full query with the inner field name substituted (so Tab correctly completes the whole expression)
- Only activate for the exact set of functions in scope — no completions inside `select(`, `map(`, `with_entries(`, or other general-filter functions

## Capabilities

### New Capabilities

- `param-field-completions`: Field-name intellisense inside the argument parens of field-path functions (`sort_by`, `group_by`, `unique_by`, `min_by`, `max_by`, `del`, `path`)

### Modified Capabilities

*(none — `json_context::get_completions` signature is unchanged; new logic is additive)*

## Impact

- `src/completions/json_context.rs` — new function `param_field_completions` called from `get_completions`
- No changes to `main.rs`, `jq_builtins.rs`, or the widget layer; completions surface through the existing pipeline
- No new dependencies
