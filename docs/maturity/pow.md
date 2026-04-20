# `pow`

**Category:** Number  
**Input type:** Number

## What's implemented

Type-aware catalog entry. Insert text `pow(.; 2)` shows the two-argument form with `.` as the base and `2` as the exponent placeholder.

## What's missing

The exponent argument is a number. Common exponents (`2`, `3`, `0.5`) could be offered as static candidates, but this has minimal practical value — users rarely look up common exponents.

## Estimated complexity

`Low` — static candidate list only.

## Needs changes to overall logic

No — could reuse any future `StaticCandidates` strategy, but the static template is sufficient.
