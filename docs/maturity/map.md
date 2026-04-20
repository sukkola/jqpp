# `map`

**Category:** Array  
**Input type:** Array

## What's implemented

Type-aware catalog entry. Insert text `map(.)` with a placeholder expression. The static template is a reasonable starting point.

## What's missing

The argument is a jq expression applied to each element. The most common patterns are:
- **Field extraction**: `map(.fieldname)` — could offer field names from the first array element (the same data source used by `sort_by`)
- **Object construction**: `map({field1, field2})` — harder to complete
- **Chained filters**: `map(select(...))` — expression composition, not completable

Field extraction is the highest-value improvement: when the cursor is at `map(.`, the system could offer field names from the first array element, exactly as `sort_by(.` does today.

## Estimated complexity

`Medium` — the field-name resolution logic already exists in `json_context.rs` for `sort_by`/`group_by`. Adding `map` to a new `FIELD_EXPR_FNS` group (distinct from `FIELD_PATH_INPUT_FNS`) that emits completions of the form `map(.fieldname)` rather than `map(.fieldname)` replace-all would reuse existing infrastructure.

## Needs changes to overall logic

Yes — `map(.` needs to be detected as a completion trigger with the cursor inside the argument expression, not just inside a string parameter. This requires extending the expression-position detection in `json_context.rs`.
