## Why

`has` and `contains` currently offer generic, type-ignoring suggestions that don't reflect how these functions actually behave — `has` takes a string key for objects but an integer index for arrays, and `contains` accepts string, array, or object arguments with completely different semantics per type. The `as $variable` binding pattern is also entirely absent from intellisense despite being fundamental to non-trivial jq queries. This change improves intellisense accuracy for these functions and introduces variable binding awareness throughout the completion system.

## What Changes

- **`has` completions**: Function acceptance inserts `has()`; live param suggestions are type-aware (`has("key")` for objects and `has(0)` for arrays).
- **`contains` completions**: Function acceptance inserts `contains()`; runtime-aware param suggestions drive string, array, and object flows (including key-first then value suggestions for object matching).
- **Duplicate `contains` entries removed**: The builtin catalog currently has two entries for `contains` (`ArrayOrObject` and `Any`); these will be collapsed into a single type-adaptive entry.
- **`as $variable` completions**: The completion engine will scan the query prefix for `as $name` bindings and offer those variable names as completions when the user types `$`.
- **Multi-type builtin architecture**: Introduce a `TypeVariant` mechanism in the builtin catalog so a single function name can carry per-type insert text, description, and completion strategy. `has` and `contains` are the first consumers; `map`, `any`, `all` (which already handle array vs. generator forms) can be migrated incrementally.
- **jaq support audit**: Document which standard jq builtins jaq 3.x does not support (`@csv`, `@tsv` are custom-implemented in executor; `debug` with message arg, `$ENV`, `label-break`, `?//` operator are absent) so the builtin catalog accurately reflects what jqpp can actually execute.

## Capabilities

### New Capabilities

- `type-aware-param-suggestions`: Completion logic that selects param-completion strategy based on runtime input type flowing into a function call. `has` and `contains` use empty-paren acceptance and type-aware param suggestions.
- `variable-binding-completions`: Parser that extracts `as $name` variable bindings from the current query prefix and surfaces matching `$name` completions when the user types `$`.

### Modified Capabilities

- `string-param-completions`: Gate `contains` full-string behavior by runtime type (`string`/unknown only). Array/object `contains` paths are handled by type-aware param suggestions.

## Impact

- `src/completions/jq_builtins.rs` — new `TypeVariant` builtin representation; updated entries for `has` and `contains`; removal of duplicate `contains` entry; jaq-unsupported functions annotated or removed
- `src/completions/json_context.rs` — type-aware param strategies for `has` and `contains`; `contains` object/array builder suggestions and runtime-type gating for string-param flow
- `src/suggestions.rs` — new variable-binding scan pass; `$` trigger for variable completions
- No breaking changes to public API or config format
- jaq 3.x crate API unchanged; no new dependencies
