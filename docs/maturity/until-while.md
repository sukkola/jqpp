# `until` / `while`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entries. Insert texts:
- `until(. > 10; . + 1)` — applies update until condition is true
- `while(. < 10; . + 1)` — applies update while condition is true

Both show a numeric iteration example.

## What's missing

The two arguments are jq expressions (a condition and an update). No data-driven completion is possible. The static examples cover the common increment/decrement pattern.

More advanced static examples could be surfaced (e.g. object-update patterns), but this has diminishing returns.

## Estimated complexity

`Low` — add alternate static examples as additional catalog entries if desired.

## Needs changes to overall logic

No.
