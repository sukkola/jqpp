# `sub` / `gsub`

**Category:** String  
**Input type:** String

## What's implemented

Type-aware catalog entry. Insert text `sub("pat"; "rep")` / `gsub("pat"; "rep")` shows the two-argument structure.

## What's missing

Both arguments are uncompletable from data in a meaningful way: the pattern is a regex, and the replacement is an arbitrary jq expression (not a static string). The replacement can reference capture groups via `\(.captures[0].string)`, making it a mini-expression.

## Estimated complexity

`High` — the replacement argument is a jq expression, not a value; completing it would require expression-aware completion which does not exist in the current architecture.

## Needs changes to overall logic

Yes — expression-aware completion inside function arguments is a fundamentally new capability.
