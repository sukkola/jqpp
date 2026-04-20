# `capture`

**Category:** String  
**Input type:** String

## What's implemented

Type-aware catalog entry. Insert text `capture("(?P<x>)")` shows the named-capture-group syntax as a hint.

## What's missing

Regex argument not completed. Named capture groups are inherently user-defined; there is no sensible way to suggest them from data.

## Estimated complexity

`High` — no practical completion strategy exists for named-capture regexes. The static template insert text is probably the best achievable.

## Needs changes to overall logic

No — the current static template approach is appropriate here.
