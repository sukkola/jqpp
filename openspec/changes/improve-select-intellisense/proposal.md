## Why

The `select()` function currently breaks the intellisense experience: triggering it skips past the argument position without offering contextual suggestions, and the default snippet fallback (`select(. != null)`) is not driven by the actual input type. Users cannot discover or compose `select()` conditions interactively.

## What Changes

- When the cursor moves inside `select(`, intellisense activates and offers contextually generated condition starters based on the inferred input type (number, string, object, array, mixed)
- The argument position inside `select(...)` is treated as a live completion trigger — Tab no longer jumps past it without offering suggestions
- Suggestions inside `select()` are derived from input type analysis, not hard-coded example strings
- For object inputs, field-based conditions (`.field == …`, `.field > …`) are suggested using keys inferred from the actual JSON input
- For string inputs, string-specific conditions (`length > …`, `startswith(…)`, `test(…)`) are offered
- For number inputs, numeric comparisons (`. > …`, `. < …`, `. % … == 0`) are offered
- For array inputs, type-dispatch suggestions (`type == "…"`) are offered
- The condition skeleton is left incomplete so the user fills in the operator and value — not pre-filled with example values

## Capabilities

### New Capabilities

- `select-intellisense`: Contextual intellisense inside `select()` arguments, driven by inferred input type from the current filter pipeline and JSON input

### Modified Capabilities

- `suggestion-activation`: The activation logic must recognize cursor-inside-select-parens as a trigger site and route to the new select-specific completion provider

## Impact

- Intellisense engine / completion provider (wherever function argument completions are currently handled)
- Input type inference logic (used to derive what kind of condition starters to offer)
- `suggestion-activation` spec behavior (new trigger condition)
- No breaking changes to the jq filter language or CLI interface
