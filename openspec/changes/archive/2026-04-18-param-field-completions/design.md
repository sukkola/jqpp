## Context

`src/completions/json_context.rs` already provides three completion sources called from `get_completions(query_prefix, input)`:
- `dot_path_completions` — fields after a bare `.`
- `obj_constructor_completions` — fields inside `{…}`
- `array_index_completions` — numeric indices inside `[…]`

`get_completions` receives the raw `query_prefix` (everything typed up to the cursor) and the full JSON input `Value`. It does not have access to the current pipe-segment type — that information lives in `main.rs`. All new logic must work from just `query_prefix` and `input`.

The functions targeted are:
| Function | Input type | Field context |
|---|---|---|
| `sort_by(.f)` | Array | First element of piped array |
| `group_by(.f)` | Array | First element of piped array |
| `unique_by(.f)` | Array | First element of piped array |
| `min_by(.f)` | Array | First element of piped array |
| `max_by(.f)` | Array | First element of piped array |
| `del(.f)` | Object | Piped object itself |
| `path(.f)` | Any | Piped value (object fields make sense) |

## Goals / Non-Goals

**Goals**
- Field name completions when cursor is inside the argument parens of the above functions
- Support simple field paths inside the parens: `.`, `.na`, `.customer.na`, etc.
- Insert-text replaces the full query prefix so Tab produces a valid, complete expression
- Works after a pipe: `.orders[] | sort_by(.n` completes `.orders[].name`-style fields

**Non-Goals**
- Completions inside `map(`, `select(`, `with_entries(`, `any(`, `all(`, `reduce`, `foreach` — these take general filters, not field paths
- Nested function calls inside the argument (`sort_by(.a | .b)`)
- Multi-argument functions (`;`-separated args) — only the first argument is a field path for all targeted functions
- Fuzzy matching inside function params (handled separately by existing fuzzy layer)

## Decisions

### D1: Where the new logic lives

Add a fourth helper `param_field_completions(query, input, out)` in `json_context.rs`, called at the end of `get_completions`. This is consistent with the existing three helpers and keeps all JSON-derived completions in one place. No changes to `main.rs` or the widget layer are needed.

**Alternative considered**: detect the pattern in `main.rs` and pass a different `json_input` to `compute_suggestions`. Rejected because it would require duplicating the parse logic and threading new state through many call sites.

### D2: Parsing strategy

Use a single pass over `query_prefix` from right to left to find the innermost unclosed `(`:
1. Walk backwards counting `)`/`(` depth.
2. When depth reaches -1, we found the opening `(`.
3. Read backwards from that `(` to grab the function name (`sort_by`, `del`, etc.).
4. The text from `(` to cursor end is the inner prefix.

This is O(n) in query length and handles nesting without a full parser. The known set of target function names is a small whitelist (`FIELD_PATH_FUNS`), so false positives are impossible.

**Alternative considered**: regex match. Rejected — harder to handle arbitrary nesting/spacing.

### D3: Resolving the field context

For array-element functions (`sort_by` etc.):
- Walk the query prefix *before* the function name to find the path that feeds into it.
- The strategy: split on `|` to get the last pipe segment before the function call; then call `find_value_at_path(input, context_path)` to resolve that path.
- If the resolved value is an `Array`, use its first element as the object to source fields from.
- If the array is empty or the first element is not an object, return no completions.

For `del` and `path`:
- The field path is relative to the current pipe-segment input, not an array element.
- Use the same `find_value_at_path` with the context path, but use the resolved value directly as the field source.

### D4: Insert-text construction

The insert-text must replace the *entire* `query_prefix` (that's the contract of `CompletionItem` in this codebase — the full text to substitute). For a query `".orders[] | sort_by(.na"` accepting `name`, the insert-text is `".orders[] | sort_by(.name"`.

Construction: `format!("{}{}", everything_up_to_inner_dot, full_inner_path)` where `everything_up_to_inner_dot` is the query prefix through the opening `(` and any path context already typed inside the parens (e.g. `.customer.`), and `full_inner_path` is the completed field path (e.g. `.customer.name`).

### D5: No changes outside json_context.rs

The existing `compute_suggestions` pipeline in `main.rs` passes `query_prefix` verbatim to `json_context::get_completions`. Since the new helper is called from within `get_completions`, no call-site changes are needed. The completions surface through the normal merge/dedup path.

## Risks / Trade-offs

- [False-positive function name match] A user-defined function with the same name as a target builtin would trigger completions. → Acceptable: jq best practice discourages shadowing builtins; the UX cost of an extra suggestion is low.
- [Context path resolution fails for complex expressions] `find_value_at_path` handles `.foo`, `.foo[]`, `.foo[0]` but not `map(.)`, `select(...)`, etc. in the path. For such queries the helper returns no completions silently. → Low risk: sort_by/group_by are almost exclusively used on simple paths.
- [del(.a,.b) multi-path] `del` can accept comma-separated paths. The parser stops at the first unclosed `(` which correctly captures the prefix of the first argument. Subsequent arguments after `,` are not completed. → Acceptable for v1.

## Migration Plan

No migration needed — purely additive, no breaking changes, no data schema changes.

## Open Questions

*(none)*
