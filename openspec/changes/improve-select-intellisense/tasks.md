## 1. Context Detection

- [x] 1.1 Add `select_condition_context` function in `completions/json_context.rs` that detects cursor inside `select(` and returns context path + inner prefix
- [x] 1.2 Add `pub fn select_condition_context(query: &str) -> Option<SelectConditionCtx>` exposing the detector (analogous to `param_field_context`)
- [x] 1.3 Add unit tests for `select_condition_context`: cursor after `select(`, cursor after partial inner text, cursor past closing `)`, nested parens inside condition

## 2. Condition Starter Generation

- [x] 2.1 Add `generate_select_starters(input_value: &Value) -> Vec<CompletionItem>` in `completions/json_context.rs` that dispatches on input type
- [x] 2.2 Implement number-type starters: `. > `, `. < `, `. == `, `. >= `, `. <= `, `. != `
- [x] 2.3 Implement string-type starters: `length > `, `startswith(`, `endswith(`, `test(`, `. == `, `. != null`
- [x] 2.4 Implement object-type starters: derive `.key > ` / `.key == ` / `.key != null` from object keys; choose operator based on value type per key; cap at a sensible limit (e.g. top 8 keys × 2–3 operators)
- [x] 2.5 Implement array-type starters: `length > `, `length == `, `. != null`
- [x] 2.6 Implement boolean / null / unknown fallback starters: `. != null`, `. == null`, `type == `
- [x] 2.7 Add unit tests for each type path in `generate_select_starters`

## 3. Prefix Filtering

- [x] 3.1 Add prefix/fuzzy filtering of condition starters by `inner_prefix` inside `generate_select_starters` or at the call site
- [x] 3.2 Verify filtering: empty prefix returns all starters; `le` narrows to `length > ` etc.; `. >` narrows to numeric comparisons

## 4. Suggestion Pipeline Integration

- [x] 4.1 Add `in_select_condition_context` detection at the top of `compute_suggestions` in `suggestions.rs`
- [x] 4.2 Add a new early-return branch for `in_select_condition_context`: evaluate context path against `eval_input`, call `generate_select_starters`, return as `Suggestion` list
- [x] 4.3 Ensure the branch falls back to infer from raw input when evaluation of context path fails
- [x] 4.4 Add integration tests in `suggestions.rs` covering: number stream, string array, object array, evaluation failure fallback

## 5. Keep-Active Wiring

- [x] 5.1 Identify where `keep_suggestions_active` / suggestion persistence is gated after Tab-acceptance of a function like `has` or `contains`
- [x] 5.2 Ensure `select` is treated equivalently — typing `select(` (cursor inside parens) keeps suggestions active and shows condition starters
- [x] 5.3 Verify Tab inside `select(` accepts a condition starter and does not jump past `)` prematurely
- [x] 5.4 Add a test or manual verification that accepting `select` from completions inserts `select()` with cursor between the parens and immediately shows condition starters

## 6. Smoke Test

- [x] 6.1 Run `cargo test` and confirm all existing tests pass
- [ ] 6.2 Manual test: input `[27.64,53.06,35.32]`, type `.[] | select(` — verify numeric starters appear
- [ ] 6.3 Manual test: input `[{"name":"Alice","age":30}]`, type `.[] | select(` — verify field-qualified starters appear (`.age > `, `.name == `)
- [ ] 6.4 Manual test: input `["apple","banana"]`, type `.[] | select(` — verify string starters appear
