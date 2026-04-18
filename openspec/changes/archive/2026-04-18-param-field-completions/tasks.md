## 1. Parser: detect param-field context

- [x] 1.1 Add `pub fn param_field_context(query: &str) -> Option<ParamFieldCtx>` in `src/completions/json_context.rs`. `ParamFieldCtx` holds: `fn_name: &str`, `context_path: &str` (the path before the function call), `inner_prefix: &str` (what is typed after `(`, e.g. `.na` or `.customer.na`). Return `None` when not inside a recognised function's parens.
- [x] 1.2 Define `FIELD_PATH_ARRAY_FNS: &[&str]` = `["sort_by", "group_by", "unique_by", "min_by", "max_by"]` and `FIELD_PATH_INPUT_FNS: &[&str]` = `["del", "path"]` as module-level constants.
- [x] 1.3 Implement the backwards scan: walk `query` right-to-left counting `)` and `(` depth; when depth goes negative, record the `(` position; then extract the function name immediately before `(`.
- [x] 1.4 If the function name is not in either constant list, return `None`.
- [x] 1.5 The `context_path` is the pipe segment before the function call; use existing `pipe_context_before` helper for extraction.
- [x] 1.6 The `inner_prefix` is the text from after `(` to the end of the query; trim leading whitespace.

## 2. Context exit detection (parser tests — heavy)

- [x] 2.1 Unit test: `param_field_context("sort_by(.name)")` returns `None` — cursor is AFTER the closing `)`, not inside
- [x] 2.2 Unit test: `param_field_context("sort_by(.name) | .")` returns `None`
- [x] 2.3 Unit test: `param_field_context("sort_by(.name).foo")` returns `None` — chained field access after closing paren
- [x] 2.4 Unit test: `param_field_context("sort_by(")` returns `Some` with empty `inner_prefix` — cursor immediately after `(`
- [x] 2.5 Unit test: `param_field_context("sort_by(.")` returns `Some` with `inner_prefix = "."`
- [x] 2.6 Unit test: `param_field_context("sort_by(.na")` returns `Some` with `inner_prefix = ".na"`
- [x] 2.7 Unit test: `param_field_context("sort_by(.customer.na")` returns `Some` with `inner_prefix = ".customer.na"`
- [x] 2.8 Unit test: `param_field_context(".orders[] | sort_by(.na")` returns `Some` with `context_path = ".orders[]"` and `inner_prefix = ".na"`
- [x] 2.9 Unit test: `param_field_context("map(.")` returns `None` — `map` is not a field-path function
- [x] 2.10 Unit test: `param_field_context("select(.")` returns `None`
- [x] 2.11 Unit test: `param_field_context("with_entries(.")` returns `None`
- [x] 2.12 Unit test: `param_field_context("del(.")` returns `Some` — `del` is in `FIELD_PATH_INPUT_FNS`
- [x] 2.13 Unit test: `param_field_context("path(.")` returns `Some`
- [x] 2.14 Unit test: `param_field_context("sort_by(.a) | group_by(.")` returns `Some` for the inner `group_by` call — nested pipe, outermost unclosed paren wins
- [x] 2.15 Unit test: `param_field_context("")` returns `None`
- [x] 2.16 Unit test: `param_field_context(".")` returns `None`
- [x] 2.17 Unit test: `param_field_context("sort_by(.name, .")` returns `Some` with inner prefix starting at the second argument — cursor is inside a multi-arg position but still inside the `(`

## 3. Completion resolver

- [x] 3.1 Add `fn param_field_completions(query: &str, input: &Value, out: &mut Vec<CompletionItem>)` in `json_context.rs`. Call `param_field_context(query)` first; if `None`, return immediately.
- [x] 3.2 For array-element functions: resolve `find_value_at_path(input, ctx.context_path)`, check it is `Value::Array`, take `first()`, check it is `Value::Object`; if any step fails, return no completions.
- [x] 3.3 For input functions (`del`, `path`): resolve `find_value_at_path(input, ctx.context_path)`, check it is `Value::Object`; if not, return no completions.
- [x] 3.4 Feed the resolved object and `ctx.inner_prefix` into the existing `dot_path_completions` logic (or inline the same key-filter loop). The prefix passed to that logic is `ctx.inner_prefix`.
- [x] 3.5 Construct `insert_text` as `format!("{}{}", query_up_to_inner_prefix_start, completed_inner_path)` so Tab replaces the whole query correctly.
- [x] 3.6 Call `param_field_completions` from `get_completions` after the existing three helpers.

## 4. Completion resolver tests — entering the context

- [x] 4.1 Test `get_completions("sort_by(.", &json!([{"name":"a","age":1}]))` returns items with labels `name` and `age`, insert-texts `sort_by(.name)` and `sort_by(.age)`
- [x] 4.2 Test `get_completions("sort_by(.na", &json!([{"name":"a","namespace":"b","age":1}]))` returns only `name` and `namespace`
- [x] 4.3 Test `get_completions("group_by(.", &json!([{"status":"x"}]))` returns `status` with insert-text `group_by(.status)`
- [x] 4.4 Test `get_completions("unique_by(.", &json!([{"id":1}]))` returns `id`
- [x] 4.5 Test `get_completions("min_by(.", &json!([{"price":1.0,"qty":5}]))` returns `price` and `qty`
- [x] 4.6 Test `get_completions("max_by(.", &json!([{"score":99}]))` returns `score`
- [x] 4.7 Test `get_completions("del(.", &json!({"name":"alice","age":30}))` returns `name` and `age` with insert-texts `del(.name)` and `del(.age)`
- [x] 4.8 Test `get_completions("del(.ag", &json!({"name":"alice","age":30}))` returns only `age`
- [x] 4.9 Test `get_completions("path(.", &json!({"a":1,"b":2}))` returns `a` and `b` with insert-texts `path(.a)` and `path(.b)`
- [x] 4.10 Test pipe context: `get_completions(".orders[] | sort_by(.", &json!({"orders":[{"id":1,"total":9.9}]}))` returns `id` and `total`, insert-texts `.orders[] | sort_by(.id)` etc.
- [x] 4.11 Test `.user | del(.", &json!({"user":{"id":1,"token":"x"}})` returns `id` and `token`, insert-texts `.user | del(.id)` etc.
- [x] 4.12 Test nested object field inside param: `get_completions("sort_by(.customer.", &json!([{"customer":{"name":"a","id":1}}]))` returns `name` and `id` with insert-texts `sort_by(.customer.name)` etc.

## 5. Completion resolver tests — exiting the context

- [x] 5.1 Test `get_completions("sort_by(.name)", ...)` returns NO param-field completions (cursor after `)`)
- [x] 5.2 Test `get_completions("sort_by(.name) | .", ...)` returns NO param-field completions
- [x] 5.3 Test `get_completions("sort_by(.name).f", ...)` returns NO param-field completions
- [x] 5.4 Test `get_completions("map(.", &json!([{"x":1}]))` returns NO param-field completions (map is excluded)
- [x] 5.5 Test `get_completions("select(.", &json!({"a":1}))` returns NO param-field completions
- [x] 5.6 Test `get_completions("with_entries(.", &json!({"a":1}))` returns NO param-field completions
- [x] 5.7 Test `get_completions("any(.", &json!([{"x":1}]))` returns NO param-field completions
- [x] 5.8 Test `get_completions("all(.", &json!([{"x":1}]))` returns NO param-field completions

## 6. Completion resolver tests — edge cases and guard conditions

- [x] 6.1 Test `get_completions("sort_by(.", &json!([]))` returns no completions (empty array)
- [x] 6.2 Test `get_completions("sort_by(.", &json!([1,2,3]))` returns no completions (scalar elements)
- [x] 6.3 Test `get_completions("sort_by(.", &json!("hello"))` returns no completions (input is not array)
- [x] 6.4 Test `get_completions("sort_by(.", &json!(null))` returns no completions
- [x] 6.5 Test `get_completions(".missing | sort_by(.", &json!({"other":[]}))` returns no completions (path not found)
- [x] 6.6 Test `get_completions("del(.", &json!([1,2]))` returns no completions (del on array, no object fields)
- [x] 6.7 Test `get_completions("del(.", &json!("hello"))` returns no completions
- [x] 6.8 Test insert-text correctness: for `get_completions("sort_by(.na", ...)`, each insert-text ends WITHOUT a closing `)` — the user finishes the expression themselves
- [x] 6.9 Test `get_completions("sort_by(.a) | group_by(.", &json!([{"x":1}]))` returns `x` from the `group_by` context, not contaminated by the closed `sort_by`
- [x] 6.10 Test that param completions are deduplicated with any other completions that might share a label (no duplicates in merged output)
