# `foreach`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entry. Insert text `foreach .[] as $x (0; . + $x)` shows the stateful-iteration pattern.

## What's missing

Same as `reduce`: variable binding completions for `$x` inside the body expression. The open spec covers this.

The optional third `EXTRACT` argument (`foreach .[] as $x (init; update; extract)`) is not surfaced.

## Estimated complexity

`Medium` — same as `reduce`.

## Needs changes to overall logic

Yes — same as `reduce`.
