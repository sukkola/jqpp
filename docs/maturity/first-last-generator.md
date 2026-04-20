# `first(expr)` / `last(expr)` — generator forms

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entries. Insert texts `first(.[])` and `last(.[])` show the generator pattern. These are distinct from the zero-argument array-element forms (`first`, `last`) which have separate `InputType::Array` entries.

## What's missing

The argument is a generator expression (anything that produces multiple outputs). Same expression-completion gap as `limit` and `map`.

## Estimated complexity

`Medium` — same as `limit` generator argument.

## Needs changes to overall logic

Yes — expression-position detection in the argument.
