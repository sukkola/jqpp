# `getpath`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entry. Insert text `getpath(["key"])` shows the path-array syntax.

## What's missing

The argument is a path array like `["a", 0, "b"]`. A useful improvement would be to offer valid path arrays derived from the live JSON structure — similar to what `path()`/`del()` do for dot-path expressions, but emitting array syntax.

Example: for input `{"user": {"name": "Alice"}}`, when cursor is at `getpath([`, suggest:
- `getpath(["user"])`
- `getpath(["user","name"])`

## Estimated complexity

`Medium` — the recursive path-walking logic already exists in `json_context.rs`. The new piece is emitting paths in array-of-strings/integers format rather than dot-notation. A new `PathArrayFns` constant alongside `FIELD_PATH_INPUT_FNS` would handle detection; path serialization needs a new formatter.

## Needs changes to overall logic

Yes — a new path serialization format (array-literal) does not currently exist in the completion system.
