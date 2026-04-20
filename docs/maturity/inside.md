# `inside`

**Category:** Array / Universal  
**Input type:** Any

## What's implemented

Two catalog entries: one `InputType::Array` with `inside([])` and one `InputType::Any` with `inside(null)`. This is the inverse of `contains` — `a | inside(b)` is equivalent to `b | contains(a)`.

## What's missing

The argument should match the type of the input: if the input is a string, the argument should be a string; if it's an array, an array; if an object, an object. Currently the insert text does not adapt to input type.

Like `contains`, real usefulness would come from offering actual data values from the live JSON as argument candidates — specifically the "containing" structures that match the current input value.

## Estimated complexity

`Medium` — same type-adaptive insert text work as `contains` (already in the spec). Data-driven argument suggestions are harder because the argument is the "outer" container, not the "inner" value.

## Needs changes to overall logic

Yes — requires the same type-adaptive insert text mechanism being built for `contains`.
