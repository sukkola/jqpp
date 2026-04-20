# `builtins` — not supported

**jq behavior:** Returns an array of all built-in function names (e.g. `["empty/0","error/1","length/0",...]`).

**Why not supported:** jaq 3.x does not implement `builtins`. Attempting to use it results in a compile error "undefined: builtins". It is intentionally absent from the completion catalog.

**Workaround:** None within jqpp. To inspect available builtins, consult the jaq documentation or source.

## Could it be added?

Only if a custom implementation were added to `executor.rs` that returns a hardcoded list of known jaq functions — similar to how `@csv`/`@tsv` are custom-implemented. This would be a static list, not a true runtime introspection.

## Estimated complexity

`Low` — implement `builtins` as a custom function in `executor.rs` returning a static slice of known function names, then add it to the catalog.

## Needs changes to overall logic

No — follows the same pattern as the `@csv`/`@tsv` custom implementations.
