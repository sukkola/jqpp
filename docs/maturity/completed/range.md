# `range` — completed

**Category:** Universal  
**Input type:** Any (generates integers, ignores input)

## What's implemented

Three `InputType::Any` catalog entries:
- `range(10)` — "0..N integer generator"
- `range(0; 10)` — "from..to integer generator"
- `range(0; 10; 2)` — "from..to step integer generator"

The dedup key in `get_completions` was changed from `name` to `(name, insert_text)` to allow all three entries to appear simultaneously.

Also present: `range()` (detail "integer generator") — the numeric builder form.

### Accept / cursor behavior

**Multi-arg catalog forms (`range(10)`, `range(0; 10)`, `range(0; 10; 2)`)**:
- NOT context-aware: "range" was removed from `FIELD_PATH_INPUT_FNS` and `is_field_path_function_call_start` because `param_field_completions` had an explicit early-return for range (its args are integers, not field paths). Keeping context-awareness caused stale suggestions to stay open and overwrite the accepted form.
- `is_numeric_builder_suggestion` does not match their detail strings
- **Enter / Tab**: inserts the full call with cursor at the **end** (after `)`)
- `keep_active = false`; dropdown closes immediately

**`range()` (numeric builder form, detail "integer generator")**:
- IS a numeric builder (`is_numeric_builder_suggestion` matches)
- **First Enter / Tab**: inserts `range()` with cursor at position 6 (inside parens); `keep_active = true`
- **Subsequent Tab** (before debounce clears suggestions): adds semicolons for multi-arg step-through

## Status

Complete. All three commonly-used pre-filled forms are discoverable and insert cleanly.

## Needs changes to overall logic

Yes — dedup key change (`name` → `(name, insert_text)` in `get_completions`).
