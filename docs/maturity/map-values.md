# `map_values`

**Category:** Array / Object  
**Input type:** ArrayOrObject

## What's implemented

Type-aware catalog entry (`InputType::ArrayOrObject`). Insert text `map_values(.)`. Works on both arrays and objects — transforms each value while preserving the container shape.

## What's missing

Same story as `map`: the argument is a jq expression. Field-name completion inside `map_values(.` would be useful for objects (suggesting key names from the current object), but the input can also be an array, making the context ambiguous.

## Estimated complexity

`Medium` — slightly more complex than `map` because the input type is `ArrayOrObject`. When the runtime type is known to be `object`, field-name completions from the object's own keys would be appropriate. When it's an array, no structural completions apply.

## Needs changes to overall logic

Yes — same expression-position detection needed as for `map`, plus type-branching on `object` vs `array` input.
