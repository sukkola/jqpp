# Other unsupported jq features

These standard jq features are absent from jqpp because jaq 3.x does not implement them. None are planned for addition.

---

## `$ENV`

**jq behavior:** A built-in object binding all environment variables. `$ENV.HOME` is equivalent to `env.HOME`.

**Why not supported:** jaq does not provide `$ENV`. Use `env.KEY` instead.

**Could it be added:** No — `$ENV` is a special variable binding at parse time, not a function. Would require parser changes.

---

## `label` / `break`

**jq behavior:** Label-break provides a non-local exit from a generator loop. Used to terminate infinite generators early without `limit`.

**Why not supported:** jaq 3.x does not implement label-break. Use `limit(n; expr)` or `first(expr)` instead.

**Could it be added:** No — requires core language changes in jaq.

---

## `?//` (alternative operator)

**jq behavior:** Try-catch shorthand. `expr ?// fallback` returns `fallback` if `expr` fails or produces no output.

**Why not supported:** Not implemented in jaq 3.x. Use `try expr catch fallback` or `expr // fallback` (for null/false) instead.

**Could it be added:** No — requires parser and compiler changes in jaq.

---

## `modulemeta`

**jq behavior:** Returns metadata about a loaded module.

**Why not supported:** jaq does not have a module system in the same sense as jq. Not applicable.

---

## `$__loc__`

**jq behavior:** Returns the current source file and line number as an object.

**Why not supported:** jaq does not expose source location information.

---

## `INDEX(stream; expr)` / `IN(s)` / `GROUP_BY(s; f)`

**jq behavior:** SQL-style operators for building lookup tables, membership tests, and grouping.

**Why not supported:** Not in jaq 3.x. Alternatives:
- `INDEX`: `reduce stream as $x ({}; . + {($x | expr): $x})`
- `IN`: `any(stream; . == input)`
- `GROUP_BY`: `group_by(f)`

---

## Streaming operators (`tostream`, `fromstream`, `truncate_stream`)

**jq behavior:** Streaming mode represents large JSON documents as a stream of `[path, value]` events to avoid loading the whole document in memory.

**Why not supported:** jaq does not implement the streaming operators. jqpp loads the entire document into memory — streaming is not applicable to the interactive TUI model.
