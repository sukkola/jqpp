## 1. Audit and clean up the builtin catalog

- [x] 1.1 Remove the duplicate `contains` entry (`InputType::Any`) from `jq_builtins.rs`; keep only the `ArrayOrObject` entry as the base
- [x] 1.2 Split `contains` into per-type catalog entries (`String` / `Array` / `Object`) while keeping function acceptance insert text as `contains()` and moving type-specific behavior to param suggestions
- [x] 1.3 Split `has` into object/array catalog entries while keeping function acceptance insert text as `has()` and moving key/index behavior to param suggestions
- [x] 1.4 Change `debug` insert text from `"debug"` (verify it has no argument placeholder) and add a jaq note to its detail field
- [x] 1.5 Update `@csv` and `@tsv` detail strings to note they are jqpp extensions (jaq does not natively support them)
- [x] 1.6 Review `input` and `inputs` entries; add "(limited in jaq)" to their detail strings

## 2. Type-aware param completions for `has`

- [x] 2.1 Add `"has"` to `FIELD_PATH_INPUT_FNS` in `json_context.rs` so object-input `has(` triggers key-name completions identical to `del`
- [x] 2.2 Handle array-input `has(` by emitting integer index completions (`has(0)`, `has(1)`, ...) up to the length of the current array input — add detection logic in `json_context.rs` alongside the existing `FIELD_PATH_INPUT_FNS` path
- [x] 2.3 Write unit tests in `json_context.rs` covering: object input all keys, object input partial filter, array input indices, after-pipe context resolution, scalar input no completions (per spec scenarios)

## 3. Type-gated `contains` string-param completions

- [x] 3.1 Thread the runtime input type into `string_param_context` (or gate the call-site in `suggestions.rs`) so that `contains(` only returns a `StringParamCtx` when input type is `string` or unknown
- [x] 3.2 Write unit tests for `string_param_context` covering: string input → `FullString`, array input → `None`, object input → `None`, unknown type → `FullString` (per spec scenarios)

## 4. Variable binding completions

- [x] 4.1 Add a function `extract_bound_variables(query_prefix: &str) -> Vec<String>` in `suggestions.rs` that scans for `as\s+\$(\w+)` patterns and returns deduplicated variable names
- [x] 4.2 In `compute_suggestions`, when the current token starts with `$`, call `extract_bound_variables` on the full query prefix and prepend matching `CompletionItem` entries (label and insert_text = `$name`, detail = `"bound variable"`) before builtin results
- [x] 4.3 Write unit tests for `extract_bound_variables`: single binding, multiple bindings, `reduce`/`foreach` forms, no bindings, empty query
- [x] 4.4 Write integration tests in `suggestions.rs` (or existing test harness) for the `$` trigger: all bound vars offered, partial prefix filters, no match returns none, ordering before builtins

## 5. End-to-end verification

- [x] 5.1 Manually verify `has(` on an object input shows live key completions in jqpp
- [x] 5.2 Manually verify `has(` on an array input shows `has(0)`, `has(1)`, etc.
- [x] 5.3 Manually verify `contains("` on a string input shows value suggestions from the live JSON
- [x] 5.4 Manually verify `contains([` and `contains({` on array/object inputs do NOT show spurious string-param suggestions and follow builder semantics
- [x] 5.5 Manually verify that after writing `.[] as $item |`, typing `$i` offers `$item` as a completion
- [x] 5.6 Run the full test suite (`cargo test`) and confirm no regressions
