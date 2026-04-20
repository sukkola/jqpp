# Intellisense Maturity Docs

One file per function (or natural group). Each file covers:
- What intellisense is currently implemented
- What is missing
- Estimated complexity to improve
- Whether it needs changes to the overall completion architecture

Complexity scale: `Low` = catalog/list change only · `Medium` = new detection or strategy · `High` = new architectural capability or fundamental limitation

---

## String functions

| File | Functions |
|---|---|
| [ascii-case](ascii-case.md) | `ascii_downcase`, `ascii_upcase` |
| [test](test.md) | `test` |
| [match](match.md) | `match` |
| [capture](capture.md) | `capture` |
| [scan](scan.md) | `scan` |
| [sub-gsub](sub-gsub.md) | `sub`, `gsub` |
| [explode](explode.md) | `explode` |
| [fromjson](fromjson.md) | `fromjson` |
| [tonumber](tonumber.md) | `tonumber` |
| [format-operators](format-operators.md) | `@base64`, `@base64d`, `@uri`, `@html`, `@sh`, `@json`, `@text`, `@csv`, `@tsv` |

## Number functions

| File | Functions |
|---|---|
| [floor-ceil-round](floor-ceil-round.md) | `floor`, `ceil`, `round` |
| [sqrt-fabs](sqrt-fabs.md) | `sqrt`, `fabs` |
| [log](log.md) | `log`, `log2`, `log10` |
| [exp](exp.md) | `exp`, `exp2`, `exp10` |
| [pow](pow.md) | `pow` |
| [isnan](isnan.md) | `isnan`, `isinfinite`, `isfinite`, `isnormal` |
| [nan-infinite](nan-infinite.md) | `nan`, `infinite` |
| [tostring](tostring.md) | `tostring` |
| [gmtime-mktime](gmtime-mktime.md) | `gmtime`, `mktime` |

## Array functions

| File | Functions |
|---|---|
| [sort](sort.md) | `sort` |
| [unique](unique.md) | `unique` |
| [reverse](reverse.md) | `reverse` |
| [add](add.md) | `add` |
| [min-max](min-max.md) | `min`, `max` |
| [map](map.md) | `map` |
| [map-values](map-values.md) | `map_values` |
| [any-all](any-all.md) | `any`, `all` |
| [first-last-nth](first-last-nth.md) | `first`, `last`, `nth` (array-element forms) |
| [transpose](transpose.md) | `transpose` |
| [implode](implode.md) | `implode` |
| [from-entries](from-entries.md) | `from_entries` |
| [inside](inside.md) | `inside` |

## Object functions

| File | Functions |
|---|---|
| [to-entries-with-entries](to-entries-with-entries.md) | `to_entries`, `with_entries` |
| [has](has.md) | `has` |

## String-or-array functions

| File | Functions |
|---|---|
| [contains](contains.md) | `contains` |
| [length](length.md) | `length` |

## Path and traversal functions

| File | Functions |
|---|---|
| [paths](paths.md) | `paths` |
| [getpath](getpath.md) | `getpath` |
| [setpath-delpaths](setpath-delpaths.md) | `setpath`, `delpaths` |
| [walk](walk.md) | `walk` |

## Control and iteration

| File | Functions |
|---|---|
| [select](select.md) | `select` |
| [limit](limit.md) | `limit` |
| [first-last-generator](first-last-generator.md) | `first(expr)`, `last(expr)` |
| [reduce](reduce.md) | `reduce` |
| [foreach](foreach.md) | `foreach` |
| [until-while](until-while.md) | `until`, `while` |
| [error](error.md) | `error` |
| [empty](empty.md) | `empty` |
| [debug](debug.md) | `debug` |

## Universal / misc

| File | Functions |
|---|---|
| [type-not](type-not.md) | `type`, `not` |
| [tojson](tojson.md) | `tojson` |
| [tostring](tostring.md) | `tostring` |
| [tonumber](tonumber.md) | `tonumber` |
| [now-env](now-env.md) | `now`, `env` |
| [null-true-false](null-true-false.md) | `null`, `true`, `false` |
| [nan-infinite](nan-infinite.md) | `nan`, `infinite` |
| [input-inputs](input-inputs.md) | `input`, `inputs` |

## Not supported by jaq 3.x

| File | Features |
|---|---|
| [not-supported/builtins](not-supported/builtins.md) | `builtins` |
| [not-supported/leaf-paths](not-supported/leaf-paths.md) | `leaf_paths` |
| [not-supported/ascii](not-supported/ascii.md) | `ascii` (codepoint → char) |
| [not-supported/recurse-down](not-supported/recurse-down.md) | `recurse_down` |
| [not-supported/format](not-supported/format.md) | `format("text")` |
| [not-supported/operators](not-supported/operators.md) | `$ENV`, `label`/`break`, `?//`, `modulemeta`, `$__loc__`, SQL-style operators, streaming |

---

## Summary: things worth doing

**39 functions moved to `completed/`** — type-aware catalog entry is the complete solution; nothing actionable remains.

**6 not-supported entries in `not-supported/`** — documented limitations of jaq 3.x.

---

### Low complexity (catalog + dedup key fix in `get_completions`)
- [`until-while`](until-while.md) — optional: more static example entries (diminishing returns)
- [`not-supported/builtins`](not-supported/builtins.md) — custom static implementation in executor
- [`not-supported/leaf-paths`](not-supported/leaf-paths.md) — jq preamble `def leaf_paths: paths(scalars);`
- [`not-supported/ascii`](not-supported/ascii.md) — jq preamble `def ascii: [.] | implode;`
- [`not-supported/recurse-down`](not-supported/recurse-down.md) — jq preamble alias (low value)

**Note on dedup key:** All functions that need multi-argument catalog entries have been moved to completed.

### Medium complexity (new detection or completion strategy)
- [`has`](has.md) — live key/index completions inside `has(` *(in open spec)*
- [`contains`](contains.md) — type-gate string-param completions *(in open spec)*
- [`reduce`](reduce.md) / [`foreach`](foreach.md) — `as $var` variable binding completions *(in open spec)*
- [`to-entries-with-entries`](to-entries-with-entries.md) — static `.key`/`.value` suggestions inside `with_entries(`
- [`map`](map.md) / [`select`](select.md) / [`any-all`](any-all.md) — field-expression completions inside argument (shared infrastructure)
- [`inside`](inside.md) — type-adaptive insert text (same work as `contains`)
- [`getpath`](getpath.md) / [`setpath-delpaths`](setpath-delpaths.md) — path-array format completions

### High complexity (new architectural capability or fundamental limitation)
- [`input-inputs`](input-inputs.md) — multi-input `DataT` wiring in executor
- [`map-values`](map-values.md) / [`first-last-generator`](first-last-generator.md) / [`limit`](limit.md) — expression-argument completions
- [`sub-gsub`](sub-gsub.md) — expression-in-replacement argument
- [`test`](test.md) / [`match`](match.md) / [`scan`](scan.md) — regex completion from data
