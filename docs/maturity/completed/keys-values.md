# `keys` / `keys_unsorted` / `values`

**Category:** Object / ArrayOrObject  
**Input type:** ArrayOrObject

## What's implemented

Type-aware catalog entries. All three are zero-argument; they appear when an object or array flows in.

Note: in jaq, `values` is defined as `select(. != null)` — it filters out null from a stream, not "extract all values from an object". This differs from standard jq's `values` (which is equivalent to `.[]` but ignoring null). The catalog entry description "values as array" is slightly inaccurate for jaq semantics.

## What's missing

Nothing completable. Zero-argument transforms.

A description accuracy fix: the `values` detail string should reflect jaq's actual behavior (`select(. != null)` — pass through non-null values).

## Estimated complexity

`Low` — detail string update only.

## Needs changes to overall logic

No.
