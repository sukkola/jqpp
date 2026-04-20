# `format("text")` — not supported

**jq behavior:** `format(fmt)` applies a named format operator to the input. `"hello" | format("@uri")` is equivalent to `"hello" | @uri`.

**Why not supported:** jaq 3.x does not implement `format/1` as a function. Format operators (`@base64`, `@uri`, etc.) are available but only as named operators, not via the `format(name)` indirection.

**Workaround:** Use the format operators directly: `@uri`, `@base64`, `@html`, etc.

## Could it be added?

Only partially — a `format/1` function could be implemented as a Rust match dispatch over the known format strings, but it would not be dynamic (could not accept a runtime-computed format string).

## Estimated complexity

`Medium` — custom Rust implementation dispatching over a fixed set of format names.

## Needs changes to overall logic

Yes — requires new custom function logic in `executor.rs`.
