# `transpose`

**Category:** Array  
**Input type:** Array (of arrays)

## What's implemented

Type-aware catalog entry. No argument needed. Flips a matrix (array of arrays) so rows become columns.

## What's missing

Nothing. Zero-argument transform. The `InputType::Array` filter is slightly over-broad (it would also appear for flat arrays), but there is no `ArrayOfArrays` type in the type system — acceptable given how rarely this matters.

## Estimated complexity

N/A — already complete.

## Needs changes to overall logic

No.
