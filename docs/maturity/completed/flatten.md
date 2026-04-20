# `flatten` — completed

**Category:** Array  
**Input type:** Array

## What's implemented

Two type-aware catalog entries:
- `flatten` (bare) — `InputType::Array`, detail "flatten nested arrays"
- `flatten(1)` — `InputType::Array`, detail "flatten N levels deep"

The dedup key in `get_completions` was changed from `name` to `(name, insert_text)` to allow both entries to appear simultaneously.

### Accept / cursor behavior

**`flatten`** (bare) uses the numeric builder path (`is_numeric_builder_suggestion` matches "flatten nested arrays"):
- **Enter**: accepts and keeps the dropdown open with depth candidates 1, 2, 3 (detail "depth")
- Selecting a depth candidate with **Enter** → writes `flatten(N)` and moves cursor past `)`
- Selecting a depth candidate with **Tab** → steps through the argument
- **Esc** while inside `flatten(` → `finalize_numeric_builder_on_escape` closes the call and moves cursor past `)`

**`flatten(1)`** (depth form) is a plain context-aware entry (`is_field_path_function_call_start` → true, detail "flatten N levels deep" does NOT match `is_numeric_builder_suggestion`):
- **Enter / Tab**: inserts `flatten(1)` with cursor at position 8 — inside the parens, before `1` — so the user can immediately edit the depth value
- `keep_active = true`, suggestions stay open
- **Esc**: closes the dropdown (no special builder finalization)

## Status

Complete. Both the bare and depth-limited forms are surfaced.
