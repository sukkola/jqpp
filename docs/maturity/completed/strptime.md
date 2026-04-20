# `strptime`

**Category:** String  
**Input type:** String

## What's implemented

Four type-aware catalog entries, all `InputType::String`:
- `strptime("%Y-%m-%d")` — "parse date string"
- `strptime("%Y-%m-%dT%H:%M:%S")` — "parse ISO datetime string"
- `strptime("%d/%m/%Y")` — "parse day/month/year string"
- `strptime("%H:%M:%S")` — "parse time string"

### Accept / cursor behavior

`strptime` is NOT in `FIELD_PATH_INPUT_FNS` or any `STRING_PARAM_*_FNS`. All entries are plain inserts:
- **Enter / Tab**: inserts the full call (e.g. `strptime("%Y-%m-%d")`) with cursor at the **end**
- `keep_active = false`
- The format string is pre-filled; the user gets the complete call and can edit if needed

This is a stopgap that improves discoverability. The proper improvement — completing format strings while the cursor is *inside* the argument parens — still requires new architecture.

## What's still missing (Medium complexity)

In-argument format-string completions require a new `StaticCandidates` strategy in `json_context.rs` — a lookup table that maps function names to static string candidate lists, triggered when the cursor is positioned inside a string argument.

**Correction from earlier assessment:** Previously documented as `Low`. The proper fix is `Medium` — requires a new strategy in `json_context.rs`. The catalog rows added here are a workaround.

The strategy would be shared between `strptime` and `strftime`, so implementing both at once is the efficient path.

## Needs changes to overall logic (for the real fix)

Yes — requires a new `StaticCandidates` param strategy (or equivalent lookup table) in `json_context.rs`.
