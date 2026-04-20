# `setpath` / `delpaths`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entries. Insert texts: `setpath(["key"]; .)` and `delpaths([["k"]])`.

## What's missing

Both functions take path arrays as their first argument. Same gap as `getpath`: the path arrays could be derived from the live JSON structure.

**`setpath`**: First argument is a path array (completable), second argument is the new value (not completable from data in a useful way).

**`delpaths`**: Argument is an array of path arrays — doubly nested. Same completion target as `getpath` but the completed value should be wrapped in an outer array.

## Estimated complexity

`Medium` — same path-array serialization work as `getpath`. `delpaths` is slightly more complex due to the outer array wrapper.

## Needs changes to overall logic

Yes — same array-path serialization format needed as `getpath`.
