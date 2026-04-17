## 1. Fuzzy Scoring Module

- [x] 1.1 Create `src/completions/fuzzy.rs` with a `fuzzy_score(token: &str, label: &str) -> Option<i32>` function: returns `None` if `token` is not a subsequence of `label` (case-insensitive), otherwise returns a score rewarding contiguous runs (+2 per adjacent matched pair) and earlier first-match position
- [x] 1.2 Add `pub fn fuzzy_completions(token: &str, items: &[CompletionItem]) -> Vec<CompletionItem>` that filters items by `fuzzy_score`, sorts descending by score, and prepends `~` to each item's `detail` field
- [x] 1.3 Add `pub mod fuzzy;` to `src/completions/mod.rs`
- [x] 1.4 Add unit tests in `src/completions/fuzzy.rs`: subsequence match, non-match, contiguous > spread scoring, earlier match > later match, empty token returns no results

## 2. Wire Fuzzy Into compute_suggestions

- [x] 2.1 In `compute_suggestions()` in `src/main.rs`, after collecting `builtin_completions` (exact prefix), call `fuzzy::fuzzy_completions(token, &all_builtins)` when `token` is non-empty
- [x] 2.2 Filter fuzzy builtin results to remove any label already present in `merged` (deduplication), then append to `merged`
- [x] 2.3 Apply the same fuzzy pass to json-context completions: after `json_completions` (exact), compute `fuzzy::fuzzy_completions(token, &all_json_fields)` and append non-duplicate results
- [x] 2.4 LSP completions: skip fuzzy pass entirely — merge as-is per existing logic

## 3. Tests

- [x] 3.1 Add an integration-style test in `tests/` (or extend existing completion tests) that calls `compute_suggestions` with a token that has no exact-prefix match and asserts fuzzy results appear with `~` in detail
- [x] 3.2 Add a test asserting exact matches appear before fuzzy matches when both are present
- [x] 3.3 Add a test asserting that an empty token produces no fuzzy candidates
