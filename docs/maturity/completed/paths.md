# `paths` — completed

**Category:** Universal  
**Input type:** Any

## What's implemented

Two `InputType::Any` catalog entries:
- `paths` (bare) — "all paths in value"
- `paths(scalars)` — "paths filtered by predicate"

The dedup key in `get_completions` was changed from `name` to `(name, insert_text)` to allow both entries to appear simultaneously.

### Accept / cursor behavior

Neither entry is in `is_field_path_function_call_start` or any string-param function list. Both are plain inserts:

- **Enter / Tab**: inserts the full text with cursor at the end — `paths` (5 chars) or `paths(scalars)` (14 chars)
- `keep_active = false`
- No in-argument completions — the predicate argument is a jq type-test expression, not a field path

## Status

Complete. Both the bare and predicate forms are discoverable via autocomplete.
