# `tonumber`

**Category:** String / Universal  
**Input type:** Any (string version converts the string; universal version is a type coercion)

## What's implemented

Two catalog entries: one `InputType::String` (converts a numeric string to a number) and one `InputType::Any` (universal coercion). Both appear at the right point in the pipe. No argument needed.

## What's missing

Nothing. Zero-argument transform.

## Estimated complexity

N/A — already complete.

## Needs changes to overall logic

No.
