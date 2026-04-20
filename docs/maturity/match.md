# `match`

**Category:** String  
**Input type:** String

## What's implemented

Type-aware catalog entry. Insert text `match("")` with cursor inside quotes.

## What's missing

Same as `test` — regex argument not completed. `match` returns a rich object `{offset, length, string, captures}` rather than a boolean, so use cases tend toward more complex regexes where literal suggestions are less useful than for `test`.

## Estimated complexity

`Medium` — same path as `test` if literal-value suggestions were ever added.

## Needs changes to overall logic

Yes — same as `test`.
