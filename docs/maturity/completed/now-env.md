# `now` / `env`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entries. Both are zero-argument generators:
- `now` returns the current UNIX timestamp as a float
- `env` returns an object of environment variables (all string values)

## What's missing

Nothing for the catalog. `env` could theoretically offer key completions from the actual environment, but environment variable names are system-dependent and not derived from the JSON input — outside the scope of jqpp's JSON-context completion model.

## Estimated complexity

N/A — already complete at the appropriate level.

## Needs changes to overall logic

No.
