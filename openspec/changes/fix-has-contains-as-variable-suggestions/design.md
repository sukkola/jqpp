## Context

The completion system has three specialised layers built on top of a flat builtin catalog:

1. **`jq_builtins.rs`** — static array of `(name, insert_text, detail, InputType)` tuples; `InputType` is used to filter suggestions by the runtime type flowing into the pipe.
2. **`json_context.rs`** — per-function-family completion strategies (field paths for `sort_by`/`del`, string values for `startswith`/`contains`, etc.).
3. **`suggestions.rs`** — orchestrator: infers runtime type, queries both layers, deduplicates.

**Current problems:**
- `has` previously lacked type-aware param behavior in the active suggestion flow, so users did not reliably get key/index completions where needed.
- `contains` had duplicate entries and string-only suggestion behavior that did not support object/array builder flows (especially after pipe evaluation and cursor-return edits).
- Variable bindings (`as $x`) appear in `reduce`/`foreach` insert text examples but the system never tracks bound names or offers `$name` completions.
- No jaq-vs-jq support annotation exists; the catalog silently lists functions (e.g., `input`, `inputs`, `debug` with arg) that jaq 3.x does not fully support.

## Goals / Non-Goals

**Goals:**
- `has` inserts as `has()` and shows live key/index completions inside its param based on runtime type.
- `contains` inserts as `contains()` and supports type-aware parameter suggestions for string, array, and object flows.
- Typing `$` after a previous `as $name` binding in the same query surfaces `$name` as a completion.
- The builtin catalog marks or omits functions not supported by jaq 3.x.

**Non-Goals:**
- Full variable scope analysis (tracking `$x` through nested `reduce`/`foreach`; resolving shadowing).
- Full semantic/query-aware ranking of object/array `contains` suggestions across deep nested structures.
- Migrating other multi-type builtins (`any`, `all`, `map`) to `TypeVariant` in this change.

## Decisions

### 1. Keep flat builtin catalog and empty-paren acceptance

Rather than introducing a new catalog type abstraction, keep the existing tuple catalog and use empty-paren function acceptance (`has()`, `contains()`). Runtime type differences are handled in param suggestion logic, not by prefilled acceptance text.

**Alternative considered**: A `TypeVariant` wrapper struct that carries `Vec<(InputType, insert_text, detail)>` per name, emitting the matching variant at query time. Rejected for this change because it requires touching the public `get_completions` return type and all callers; the two-entry approach achieves the same user-visible result with local edits.

### 2. `has` param completions via existing `FIELD_PATH_INPUT_FNS`

`json_context.rs` already handles `del` and `path` as "field path input functions" — functions whose argument is a dot-path into the *current* input (not an array element). `has` has identical semantics: for objects it takes a field name; for arrays it takes an index. Add `has` to `FIELD_PATH_INPUT_FNS` (object input) and handle the integer-index case alongside `.[N]` completions for array input.

### 3. `contains` uses gated string strategy plus object/array builder suggestions

The existing `FullString` extraction strategy remains for string/unknown input types. For array/object input types, `contains` routes to dedicated param-field logic with builder semantics:
- object flow: suggest keys first, then values for the selected key
- array flow: suggest scalar elements progressively
- edit/return flow: continue suggesting correctly when cursor returns inside an existing `contains(...)` argument

**Alternative**: Remove `contains` from `STRING_PARAM_FULLSTRING_FNS` entirely and handle it in a new `type_aware_param_context` function. Rejected because that duplicates working logic and breaks the string case without adding value.

### 4. Variable binding scan in `suggestions.rs`

When the current token starts with `$`, scan the full query prefix for occurrences of `as\s+\$(\w+)` using a regex (or simple pattern match). Collect all matched names, deduplicate, and emit them as `CompletionItem` entries with `insert_text = $name` and `detail = "bound variable"`. These are prepended to the suggestion list before builtins.

This is intentionally shallow: it does not parse scopes, does not distinguish `reduce`/`foreach` inner scope from outer scope, and does not track variable type. These limitations are acceptable because: (a) most real queries bind one or two variables with distinct names; (b) offering a false positive is much less harmful than missing a real binding.

### 5. jaq support annotation

Add a `jaq_supported: bool` field (or a comment block) to the builtin catalog. Functions confirmed unsupported by jaq 3.x will be annotated or removed:
- `@csv`, `@tsv` — custom-implemented in `executor.rs`; keep in catalog, note in detail that these are jqpp extensions.
- `debug` with message argument — jaq 3.x `debug` takes no argument; remove or restrict insert text to bare `debug`.
- `input`, `inputs` — partially supported; keep with a "(limited)" note.
- `$ENV` — not a function entry, not relevant here.
- `label-break` — not in catalog; no action needed.

## Risks / Trade-offs

- **Two-entry approach for `contains`**: If a future `TypeVariant` refactor is done, the per-type entries must be merged. Low risk — the pattern is already used elsewhere in the catalog.
- **Shallow variable scan false positives**: A `$x` bound inside a `reduce` that is already closed will still be suggested after the closing `)`. Acceptable for now; the fix is a proper scope parser which is out of scope.
- **`has` array index completions**: indices must remain numeric and filtered by typed prefix.
- **`contains` builder edit states**: Tab/Enter/Esc must keep query syntax valid when user returns to edit an existing argument.
