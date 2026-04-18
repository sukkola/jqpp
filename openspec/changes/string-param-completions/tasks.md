## 1. StringParamStrategy and StringParamCtx types

- [x] 1.1 Add `pub enum StringParamStrategy { Prefix, Suffix, Internal, FullString }` in `src/completions/json_context.rs`
- [x] 1.2 Add `pub struct StringParamCtx<'a> { pub fn_name: &'a str, pub strategy: StringParamStrategy, pub context_path: &'a str, pub inner_prefix: &'a str }` in the same file
- [x] 1.3 Define `STRING_PARAM_PREFIX_FNS: &[&str] = &["startswith", "ltrimstr"]`
- [x] 1.4 Define `STRING_PARAM_SUFFIX_FNS: &[&str] = &["endswith", "rtrimstr"]`
- [x] 1.5 Define `STRING_PARAM_INTERNAL_FNS: &[&str] = &["split"]`
- [x] 1.6 Define `STRING_PARAM_FULLSTRING_FNS: &[&str] = &["contains", "index", "rindex", "indices"]`

## 2. string_param_context detector

- [x] 2.1 Add `pub fn string_param_context(query: &str) -> Option<StringParamCtx>` in `json_context.rs`. Walk `query` right-to-left counting `)` / `(` depth; when depth goes negative, record the open-paren position and extract the function name before it. Return `None` if function name is not in any of the four constant lists.
- [x] 2.2 Extract `context_path` using `pipe_context_before` on the text before the function name.
- [x] 2.3 Extract `inner_prefix` as the text from `(` to end of query; if it starts with `"`, strip the leading quote. Trim leading whitespace.
- [x] 2.4 Assign `strategy` by checking which constant list the function name falls in.

## 3. Detector unit tests — entering the context

- [x] 3.1 Test `string_param_context("split(")` → `Some` with `fn_name="split"`, `strategy=Internal`, `context_path="."`, `inner_prefix=""`
- [x] 3.2 Test `string_param_context("split(\"")` → `Some` with `inner_prefix=""` (quote stripped)
- [x] 3.3 Test `string_param_context("split(\"-")` → `Some` with `inner_prefix="-"`
- [x] 3.4 Test `string_param_context("startswith(\"shi")` → `Some` with `strategy=Prefix`, `inner_prefix="shi"`
- [x] 3.5 Test `string_param_context("endswith(\".com")` → `Some` with `strategy=Suffix`, `inner_prefix=".com"`
- [x] 3.6 Test `string_param_context("ltrimstr(")` → `Some` with `strategy=Prefix`
- [x] 3.7 Test `string_param_context("rtrimstr(")` → `Some` with `strategy=Suffix`
- [x] 3.8 Test `string_param_context("contains(")` → `Some` with `strategy=FullString`
- [x] 3.9 Test `string_param_context("index(")` → `Some` with `strategy=FullString`
- [x] 3.10 Test `string_param_context("rindex(")` → `Some` with `strategy=FullString`
- [x] 3.11 Test `string_param_context("indices(")` → `Some` with `strategy=FullString`
- [x] 3.12 Test pipe context: `string_param_context(".orders[].order_status | split(\"_")` → `context_path=".orders[].order_status"`, `inner_prefix="_"`
- [x] 3.13 Test `string_param_context("map(split(\"")` → `Some` — inner `split` is recognised; `context_path` resolves from the closest pipe context

## 4. Detector unit tests — exiting the context

- [x] 4.1 Test `string_param_context("split(\"-\")")` → `None` — cursor after closing `)`
- [x] 4.2 Test `string_param_context("split(\"-\") | .")` → `None`
- [x] 4.3 Test `string_param_context("split(\"-\").foo")` → `None`
- [x] 4.4 Test `string_param_context("test(\"")` → `None` — excluded regex function
- [x] 4.5 Test `string_param_context("match(\"")` → `None`
- [x] 4.6 Test `string_param_context("scan(\"")` → `None`
- [x] 4.7 Test `string_param_context("sub(\"")` → `None`
- [x] 4.8 Test `string_param_context("gsub(\"")` → `None`
- [x] 4.9 Test `string_param_context("capture(\"")` → `None`
- [x] 4.10 Test `string_param_context("strptime(\"")` → `None`
- [x] 4.11 Test `string_param_context("strftime(\"")` → `None`
- [x] 4.12 Test `string_param_context("")` → `None`
- [x] 4.13 Test `string_param_context(".")` → `None`
- [x] 4.14 Test `string_param_context("sort_by(.")` → `None` — sort_by is a field-path function, not a string-param function

## 5. String value collection

- [x] 5.1 Add `fn collect_string_values<'a>(val: &'a serde_json::Value) -> Vec<&'a str>` that returns string scalars from a `Value::String`, `Value::Array` (string elements only), or `Value::Object` (string values only); returns empty vec for all other types. Cap at 500 entries.
- [x] 5.2 Test: scalar string → single-element vec
- [x] 5.3 Test: array of mixed types → only string elements returned
- [x] 5.4 Test: object with mixed values → only string values returned
- [x] 5.5 Test: number scalar → empty vec
- [x] 5.6 Test: null → empty vec
- [x] 5.7 Test: array of 1000 strings → vec capped at 500

## 6. Candidate extraction — Prefix strategy

- [x] 6.1 Add `fn extract_prefix_candidates(strings: &[&str]) -> Vec<String>`. Tokenise each string on `[\\-_/. @]`, collect the leading token and leading-token+first-delimiter pairs, plus the full string. Deduplicate and sort.
- [x] 6.2 Test: `["CUST-42", "CUST-17"]` → includes `"CUST"` and `"CUST-"`
- [x] 6.3 Test: `["shipped", "processing", "delivered"]` → includes each full word (no delimiters)
- [x] 6.4 Test: empty input → empty vec
- [x] 6.5 Test: result is always sorted and deduplicated

## 7. Candidate extraction — Suffix strategy

- [x] 7.1 Add `fn extract_suffix_candidates(strings: &[&str]) -> Vec<String>`. Tokenise from the right (last delimiter-bounded token), collect trailing-token and delimiter+trailing-token combos, plus full string. Deduplicate and sort.
- [x] 7.2 Test: `["alice@example.com", "mikko@example.com"]` → includes `".com"`, `"example.com"`, `"@example.com"`
- [x] 7.3 Test: `["shipped", "delivered"]` → includes each full word
- [x] 7.4 Test: empty input → empty vec
- [x] 7.5 Test: result is sorted and deduplicated

## 8. Candidate extraction — Internal strategy

- [x] 8.1 Add `fn extract_internal_candidates(strings: &[&str]) -> Vec<String>`. For each non-alphanumeric character, count how many strings contain it. Include single-char delimiters that appear in ≥ 2 strings. Also collect multi-char delimiter runs between alphanumeric runs and include those that recur in ≥ 2 strings. Deduplicate and sort.
- [x] 8.2 Test: `["CUST-42", "ORD-001", "STORE-001"]` → includes `"-"`
- [x] 8.3 Test: single-char delimiter appearing in only 1 string is excluded
- [x] 8.4 Test: multi-char delimiter `"__"` appearing in ≥ 2 strings is included
- [x] 8.5 Test: empty input → empty vec
- [x] 8.6 Test: strings with no delimiters → empty vec

## 9. Candidate extraction — FullString strategy

- [x] 9.1 Add `fn extract_fullstring_candidates(strings: &[&str]) -> Vec<String>`. Return unique sorted copies of the input strings.
- [x] 9.2 Test: `["shipped", "processing", "shipped"]` → `["processing", "shipped"]`
- [x] 9.3 Test: empty input → empty vec

## 10. Completion resolver

- [x] 10.1 Add `fn string_param_completions(query: &str, input: &Value, out: &mut Vec<CompletionItem>)` in `json_context.rs`. Call `string_param_context(query)` first; if `None`, return immediately.
- [x] 10.2 Resolve `find_value_at_path(input, ctx.context_path)`, then call `collect_string_values` on the result. If empty, return.
- [x] 10.3 Dispatch to the appropriate extraction function based on `ctx.strategy`.
- [x] 10.4 Filter candidates: keep only those where `candidate.starts_with(ctx.inner_prefix)`. If `inner_prefix` is non-empty and no prefix matches, fall back to `fuzzy_score` over all candidates (reusing `src/completions/fuzzy.rs`).
- [x] 10.5 Construct `insert_text` as `format!("{}\"{}\"", query_up_to_open_paren, escaped_candidate)` where `escaped_candidate` replaces `"` with `\\"`.
- [x] 10.6 Cap results at 10 items.
- [x] 10.7 Call `string_param_completions` from `get_completions` after `param_field_completions`.

## 11. Completion resolver integration tests — entering context

- [x] 11.1 Test `get_completions("split(", &json!(["a-b", "c-d"]))` → contains item with label `"-"` and insert-text `"split(\"-\")"`
- [x] 11.2 Test `get_completions("split(\"-", &json!(["a-b", "c-d"]))` → contains `"-"` candidate
- [x] 11.3 Test `get_completions("startswith(", &json!(["shipped", "processing"]))` → contains `"shipped"` and `"processing"` candidates; insert-texts `startswith("shipped")` etc.
- [x] 11.4 Test `get_completions("startswith(\"sh", &json!(["shipped", "processing"]))` → only `"shipped"` (prefix filter `"sh"`)
- [x] 11.5 Test `get_completions("endswith(", &json!(["alice@example.com"]))` → contains `.com` and `@example.com` suffix candidates
- [x] 11.6 Test `get_completions("ltrimstr(", &json!(["CUST-42", "CUST-17"]))` → contains `"CUST"` and `"CUST-"`
- [x] 11.7 Test `get_completions("rtrimstr(", &json!(["alice@example.com"]))` → contains trailing token candidates
- [x] 11.8 Test `get_completions("contains(", &json!(["shipped", "processing"]))` → contains full strings as candidates
- [x] 11.9 Test `get_completions("index(", &json!(["shipped"]))` → contains `"shipped"`
- [x] 11.10 Test pipe context: `get_completions(".orders[].order_status | split(", &json!({"orders": [{"order_status": "ship-fast"}, {"order_status": "plan-ahead"}]}))` → contains `"-"` as separator candidate

## 12. Completion resolver integration tests — exiting context

- [x] 12.1 Test `get_completions("split(\"-\")", &json!(["a-b"]))` → NO string-param completions
- [x] 12.2 Test `get_completions("split(\"-\") | .", &json!(["a-b"]))` → NO string-param completions
- [x] 12.3 Test `get_completions("test(", &json!(["foo"]))` → NO string-param completions
- [x] 12.4 Test `get_completions("match(", &json!(["foo"]))` → NO string-param completions
- [x] 12.5 Test `get_completions("gsub(", &json!(["foo"]))` → NO string-param completions
- [x] 12.6 Test `get_completions("strptime(", &json!(["2024-01-01"]))` → NO string-param completions

## 13. Completion resolver edge-case tests

- [x] 13.1 Test `get_completions("split(", &json!([]))` → no completions (empty array)
- [x] 13.2 Test `get_completions("split(", &json!([1, 2, 3]))` → no completions (no string values)
- [x] 13.3 Test `get_completions("split(", &json!(42))` → no completions
- [x] 13.4 Test `get_completions("split(", &json!(null))` → no completions
- [x] 13.5 Test `get_completions(".missing | split(", &json!({"other": ["x"]}))` → no completions (path not found)
- [x] 13.6 Test insert-text escapes embedded double quotes: source string `"say \"hi\""` → insert-text uses `\\"` so jq expression is valid
- [x] 13.7 Test no duplicate labels in merged output when string-param candidates overlap with other completion sources
- [x] 13.8 Test fuzzy fallback: `get_completions("startswith(\"shiped", &json!(["shipped"]))` → fuzzy matches `"shipped"` even though `"shiped"` is not an exact prefix

## 14. Builtin insert-text changes

- [x] 14.1 In `src/completions/jq_builtins.rs`, change `split` insert-text from `split(\",\")` to `split()`
- [x] 14.2 Change `startswith` insert-text from `startswith(\"\")` to `startswith()`
- [x] 14.3 Change `endswith` insert-text from `endswith(\"\")` to `endswith()`
- [x] 14.4 Change `ltrimstr` insert-text from `ltrimstr(\"\")` to `ltrimstr()`
- [x] 14.5 Change `rtrimstr` insert-text from `rtrimstr(\"\")` to `rtrimstr()`
- [x] 14.6 Change `contains` (string/array variants) insert-text to `contains()`
- [x] 14.7 Change `index` insert-text from `index(\"\")` to `index()`
- [x] 14.8 Change `rindex` insert-text from `rindex(\"\")` to `rindex()`
- [x] 14.9 Change `indices` insert-text from `indices(\"\")` to `indices()`
- [x] 14.10 Add/update unit tests in `jq_builtins.rs` asserting each changed function has `insert_text == "fn()"` format

## 15. cursor_col_after_accept update

- [x] 15.1 In `src/main.rs`, update `cursor_col_after_accept` to also handle the empty-parens pattern `fn()`: when insert-text ends with `()`, place cursor at `len - 1` (inside the parens). Ensure the existing `("` pattern still works for functions that retain placeholder quoted args.
- [x] 15.2 Add a unit test: `cursor_col_after_accept("split()")` returns the index of the `(` + 1
- [x] 15.3 Add a unit test: `cursor_col_after_accept("startswith()")` returns position inside the parens

## 16. Iterative refinement: ordering, suffix growth, and Tab/Enter UX

- [x] 16.1 Update delimiter tokenization to include space, tab, comma, dot, pipe, and hyphen boundaries for string-param candidate extraction
- [x] 16.2 Refine prefix extraction to avoid delimiter-attached duplicates (e.g., keep `CUST`, drop `CUST-`)
- [x] 16.3 Refine suffix extraction to provide layered right-to-left candidates (`com` -> `corp.com` -> `@corp.com`)
- [x] 16.4 Make string-param filtering strategy-aware (`ends_with` for suffix functions, `starts_with` for others)
- [x] 16.5 Order exact candidates shortest-first and append fuzzy matches at the end with `~` detail
- [x] 16.6 Implement Tab extension semantics for suffix contexts so repeated Tab extends from short suffix to longer suffix chains
- [x] 16.7 Keep Enter commit semantics for partial typed values and validate cursor placement after commit
- [x] 16.8 Add thorough tests for tokenization boundaries, suffix layering, ordering, fuzzy-after-exact, and repeated Tab extension behavior

## 17. Format Operator Restrictions (@tsv, @csv)

- [x] 17.1 Add `InputType::ArrayOfScalars` and update `compatible_with` in `src/completions/jq_builtins.rs`
- [x] 17.2 Update `@csv`, `@tsv`, and `implode` built-ins to use `InputType::ArrayOfScalars`
- [x] 17.3 Update `jq_type_of` to return `"array_scalars"` for arrays containing only scalars
- [x] 17.4 Add unit tests verifying `@tsv` is excluded for arrays of objects and included for arrays of scalars

## 18. Pipe Prefix Evaluation for Suggestions

- [x] 18.1 Add `split_at_last_pipe` helper in `src/main.rs`
- [x] 18.2 Update `compute_suggestions` to evaluate the query prefix before the last pipe
- [x] 18.3 Use the evaluated value as context for JSON and fuzzy field completions
- [x] 18.4 Add regression tests for complex pipe chains and nested field access
