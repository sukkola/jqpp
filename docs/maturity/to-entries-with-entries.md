# `to_entries` / `with_entries`

**Category:** Object  
**Input type:** Object (also ArrayOrObject for `to_entries`)

## What's implemented

Type-aware catalog entries. `to_entries` is present for both `InputType::Object` and `InputType::ArrayOrObject`. `with_entries(.value)` has a static expression placeholder.

## What's missing

**`to_entries`**: Zero-argument transform — nothing missing.

**`with_entries(f)`**: The argument is a jq expression over `{key, value}` entries. Useful completions would suggest `.key` and `.value` field access since those are always the available fields inside `with_entries`. This is simpler than generic expression completion because the available fields are always exactly `key` and `value`.

## Estimated complexity

`Low` for `with_entries` — the available fields are fixed (`key`, `value`), so a static field list `[".key", ".value"]` could be injected whenever the cursor is inside `with_entries(.`. No JSON walking needed.

## Needs changes to overall logic

Yes — detecting the cursor inside `with_entries(` and offering a fixed field list is a small new detection case in `json_context.rs`.
