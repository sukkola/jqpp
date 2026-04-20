# `any` / `all`

**Category:** Array  
**Input type:** Array

## What's implemented

Type-aware catalog entries. Insert texts `any(. > 0)` and `all(. > 0)` show a predicate example. Both also have two-argument forms `any(generator; condition)` / `all(generator; condition)` that are not surfaced separately.

## What's missing

The argument is a boolean predicate expression. Useful completions would be field comparisons derived from the array elements (e.g. `any(.status == "active")`). This is the same field-expression completion gap as `map` and `select`.

The two-argument generator forms are also absent from the catalog.

## Estimated complexity

`Medium` — field-expression completions inside the predicate argument; same approach as `map`.

## Needs changes to overall logic

Yes — expression-position detection in the argument, same as `map`.
