# `first` / `last` / `nth`

**Category:** Array  
**Input type:** Array

## What's implemented

Type-aware catalog entries for the zero-argument array-element forms (`first`, `last`, `nth(0)`). These return the first, last, or Nth element of the array.

The one-argument generator forms (`first(expr)`, `last(expr)`) also have separate `InputType::Any` catalog entries — see `first-last-generator.md`.

## What's missing

For `nth(N)`: the index argument is a literal integer. No data-driven completion is possible or needed.

## Estimated complexity

N/A — already complete for the array-element forms.

## Needs changes to overall logic

No.
