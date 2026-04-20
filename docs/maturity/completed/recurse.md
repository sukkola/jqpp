# `recurse` — completed

**Category:** Universal  
**Input type:** Any

## What's implemented

Two `InputType::Any` catalog entries:
- `recurse` (bare) — "recursive descent"
- `recurse(.[]?)` — "safe recursive descent (error-suppressed)"

The dedup key in `get_completions` was changed from `name` to `(name, insert_text)` to allow both entries to appear simultaneously.

### Accept / cursor behavior

Neither entry is in `is_field_path_function_call_start` or any string-param function list. Both are plain inserts:

- **Enter / Tab**: inserts the full text with cursor at the end — `recurse` (7 chars) or `recurse(.[]?)` (13 chars)
- `keep_active = false`
- No in-argument completions — the argument is a jq filter expression, not a field path or string

The `?` error-suppressor in `recurse(.[]?)` makes it safe for mixed-type JSON trees where some nodes are not arrays or objects; `.[]` on a scalar would otherwise error.

## Status

Complete. Both the bare and safe-argument forms are discoverable via autocomplete.
