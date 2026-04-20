# `implode`

**Category:** Array  
**Input type:** ArrayOfScalars (array of integer codepoints)

## What's implemented

Type-aware catalog entry with `InputType::ArrayOfScalars`. Appears only when the array contains no nested arrays or objects (all elements are scalars). No argument needed; returns the string formed by treating each integer as a Unicode codepoint.

## What's missing

Nothing. Zero-argument transform. The `ArrayOfScalars` type filter is the best available approximation — it correctly suppresses the suggestion for arrays of objects or nested arrays.

## Estimated complexity

N/A — already complete.

## Needs changes to overall logic

No.
