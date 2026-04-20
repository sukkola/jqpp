# `select`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entry. Insert text `select(. != null)` shows a common predicate.

## What's missing

The argument is a boolean predicate expression. The most common real-world patterns are field comparisons:
- `select(.status == "active")`
- `select(.age > 18)`
- `select(.type == "user")`

When the cursor is at `select(.`, field names from the current input object could be suggested, producing completions like `select(.fieldname`. This is the same expression-position detection needed by `map` and `any`/`all`.

## Estimated complexity

`Medium` — shares the field-expression detection infrastructure with `map`. The difference is that `select` works on the current input directly (not on array elements), so the field source is the current input rather than the first array element.

## Needs changes to overall logic

Yes — expression-position detection inside `select(` argument; same architectural addition as `map`.
