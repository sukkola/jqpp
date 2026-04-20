# `walk`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entry. Insert text `walk(if type == "array" then sort else . end)` shows a realistic example that sorts nested arrays.

## What's missing

The argument is a jq expression applied depth-first to every node. There is no practical data-driven completion — the expression is structural and user-defined. The existing static example is the best achievable completion hint.

## Estimated complexity

`High` — expression-aware completion inside `walk` would require the same general expression-completion infrastructure as `map`/`select`. The static example is the realistic ceiling.

## Needs changes to overall logic

No — the static template approach is appropriate here.
