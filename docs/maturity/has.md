# `has`

**Category:** Object / Array  
**Input type:** Object or Array

## What's implemented

Two type-adaptive catalog entries:
- `InputType::Object` → insert text `has("key")`, detail "test object key presence"
- `InputType::Array` → insert text `has(0)`, detail "test array index presence"

The correct insert text is surfaced based on runtime type.

### Accept / cursor behavior

Both forms are context-aware (`is_field_path_function_call_start` → true, "has" is in `FIELD_PATH_INPUT_FNS`):

**`has("key")` (object form)**:
- **Enter / Tab**: inserts `has("key")` with cursor at position 5 — after `has("`, before `key` — so the user can immediately type their own key name
- `keep_active = true`; the param-field context for "has" with object input offers live key completions from the JSON data
- **Esc**: closes the dropdown; no builder finalization

**`has(0)` (array form)**:
- **Enter / Tab**: inserts `has(0)` with cursor at position 4 — inside the parens, before `0`
- `keep_active = true`; the param-field context for "has" does not generate integer-index completions (not yet implemented — see below)
- **Esc**: closes the dropdown

## What's missing

**Live param completions** — when the cursor is inside `has(`, the argument should be completed from actual data:
- Object input: suggest actual key names from the current object (e.g. `has("name")`, `has("age")`)
- Array input: suggest valid indices (e.g. `has(0)`, `has(1)`, `has(2)`)

Object key completion is in the open spec (`fix-has-contains-as-variable-suggestions`, task 2.1–2.3). Array index completion requires a small new branch in `param_field_context`.

## Estimated complexity

`Medium` — the object key-name path is straightforward: add `"has"` to `FIELD_PATH_INPUT_FNS` in `json_context.rs`. The array index path requires a small new branch that emits integer-index completions up to the array length.

## Needs changes to overall logic

Yes — array index completions are a new variant alongside the existing key-name completion path. Small, isolated change.
