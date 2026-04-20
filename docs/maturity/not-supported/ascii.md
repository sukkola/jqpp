# `ascii` (codepoint → char) — not supported

**jq behavior:** Takes an integer codepoint and returns a single-character string. `65 | ascii` → `"A"`.

**Why not supported:** jaq 3.x does not provide an `ascii/0` function that converts a number to a character. Attempting to use it fails with a compile error.

**Workaround:** `[.] | implode` — wrap the codepoint in an array and use `implode`, which converts an array of codepoints to a string.

## Could it be added?

Yes — `def ascii: [.] | implode;` as a preamble definition or a custom Rust function.

## Estimated complexity

`Low` — one-line jq definition injectable as a preamble.

## Needs changes to overall logic

Yes — requires the query preamble mechanism described in `leaf-paths.md`.
