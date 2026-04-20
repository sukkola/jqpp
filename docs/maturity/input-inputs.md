# `input` / `inputs`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entries with detail strings noting "(limited in jaq)". Both appear as completion suggestions.

## What's missing

**Runtime support**: jaq provides `input`/`inputs` through a `HasInputs` trait, but jqpp's executor uses `JustLut` as its `DataT` which does not implement `HasInputs`. As a result, both functions fail at runtime with a compile/undefined error in jaq's filter compiler.

The catalog entries show the functions as available but they cannot currently be executed. Either:
1. The entries should be removed and moved to the "not supported" list, or
2. jqpp should wire up `jaq_std::input::funs()` with a `DataT` that implements `HasInputs`, providing actual multi-input support.

Option 2 would require significant architectural work since jqpp is currently a single-input tool.

## Estimated complexity

`High` — wiring multi-input support requires redesigning the `Executor` to accept an iterator of input values and a compatible `DataT` implementation.

## Needs changes to overall logic

Yes — requires a new `DataT` implementation that carries an input iterator.
