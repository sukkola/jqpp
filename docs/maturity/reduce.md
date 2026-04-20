# `reduce`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entry. Insert text `reduce .[] as $x (0; . + $x)` shows the accumulator pattern with a variable binding.

## What's missing

**Variable binding completions**: After `reduce .[] as $x (0;`, typing `$x` should offer `$x` as a completion. The system currently does not track `as $name` bindings or offer `$name` completions anywhere. This is the `variable-binding-completions` capability in the open spec (`fix-has-contains-as-variable-suggestions`, tasks 4.1–4.4).

**Generator expression**: The `.[]` generator part could use expression-position completions, but the static example covers the most common case.

## Estimated complexity

`Medium` — variable binding completions require a regex scan of the query prefix for `as $name` patterns, then surfacing matching names when `$` is typed. Described in detail in the open spec.

## Needs changes to overall logic

Yes — new variable-name tracking pass in `suggestions.rs`; new `$` trigger for completions.
