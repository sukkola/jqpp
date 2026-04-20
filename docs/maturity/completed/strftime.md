# `strftime`

**Category:** Number  
**Input type:** Number (UNIX timestamp)

## What's implemented

Four type-aware catalog entries, all `InputType::Number`:
- `strftime("%Y-%m-%d")` — "format UNIX time"
- `strftime("%Y-%m-%dT%H:%M:%SZ")` — "format as ISO datetime"
- `strftime("%H:%M:%S")` — "format as time"
- `strftime("%Y/%m/%d %H:%M")` — "format as date and time"

### Accept / cursor behavior

`strftime` is NOT in `FIELD_PATH_INPUT_FNS` or any `STRING_PARAM_*_FNS`. All entries are plain inserts:
- **Enter / Tab**: inserts the full call (e.g. `strftime("%Y-%m-%d")`) with cursor at the **end**
- `keep_active = false`
- The format string is pre-filled; the user gets the complete call and can edit if needed

This is a stopgap that improves discoverability. The proper improvement — completing format strings while the cursor is *inside* the argument parens — still requires new architecture.

## What's still missing (Medium complexity)

In-argument format-string completions require a new `StaticCandidates` strategy in `json_context.rs`, shared with `strptime`.

**Correction from earlier assessment:** Previously documented as `Low`. The proper fix is `Medium`.

## Needs changes to overall logic (for the real fix)

Yes — requires a new `StaticCandidates` param strategy (or equivalent lookup table) in `json_context.rs`.
