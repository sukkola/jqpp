# `limit`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entry. Insert text `limit(10; .[])` shows the count + generator pattern.

## What's missing

The first argument is a count (integer literal). Common values like `1`, `5`, `10`, `100` could be offered as static completions, but this has minimal value — users know what count they want.

The second argument is a generator expression — same expression-completion gap as `map`/`select`.

## Estimated complexity

`Low` for static count completions; `Medium` for generator expression completions (same path as `map`).

## Needs changes to overall logic

No for static count completions; Yes for generator expression completions.
