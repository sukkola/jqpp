# `length`

**Category:** Universal (except boolean)  
**Input type:** NonBoolean (string, number, array, object, null — NOT boolean)

## What's implemented

`InputType::NonBoolean` catalog entry. This correctly excludes `length` from appearing after boolean-producing expressions (`not`, `test`, `endswith`, etc.) because `true | length` is a jq runtime error.

No argument needed; returns the character count, element count, key count, absolute value, or 0 for null.

## What's missing

Nothing. Zero-argument transform with correct type filtering.

## Estimated complexity

N/A — already complete.

## Needs changes to overall logic

No.
