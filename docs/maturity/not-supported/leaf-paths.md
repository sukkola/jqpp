# `leaf_paths` — not supported

**jq behavior:** Returns all paths to leaf (non-container) values. Equivalent to `[path(..| scalars)]` or `paths(scalars)`.

**Why not supported:** jaq 3.x does not define `leaf_paths`. Attempting to use it results in a compile error.

**Workaround:** `paths(scalars)` — jaq does support `paths` with a predicate filter, and `scalars` is defined in jaq-std.

## Could it be added?

Yes — as a user-defined jq function injected alongside the query, or as a macro expansion in `executor.rs`. The simplest approach is to add `def leaf_paths: paths(scalars);` as a preamble to every query.

## Estimated complexity

`Low` — add `def leaf_paths: paths(scalars);` to a query preamble in `executor.rs` and add a catalog entry.

## Needs changes to overall logic

Yes — the executor would need to prepend standard definitions not provided by jaq, which is a new concept. Alternatively, emit a completion that expands to `paths(scalars)` instead.
