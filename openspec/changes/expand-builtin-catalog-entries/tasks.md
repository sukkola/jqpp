## 1. Fix dedup key in get_completions

- [x] 1.1 In `src/completions/jq_builtins.rs`, change `seen.insert(name)` to `seen.insert((name, insert_text))` in `get_completions`
- [x] 1.2 Update the existing `no_duplicates_in_output` test to check uniqueness of `(label, insert_text)` pairs instead of `label` alone (the old assertion will fail once same-name entries are added)
- [x] 1.3 Run `cargo test` to confirm the dedup change compiles and existing tests pass before adding new entries

## 2. Add multi-argument catalog entries

- [x] 2.1 Add `("flatten", "flatten(1)", "flatten N levels deep", InputType::Array)` to `BUILTINS` immediately after the existing bare `flatten` entry
- [x] 2.2 Add `("range", "range(0; 10)", "from..to integer generator", InputType::Any)` after the existing `range(10)` entry
- [x] 2.3 Add `("range", "range(0; 10; 2)", "from..to step integer generator", InputType::Any)` after the `range(0; 10)` entry
- [x] 2.4 Add `("paths", "paths(scalars)", "paths filtered by predicate", InputType::Any)` after the existing bare `paths` entry
- [x] 2.5 Add `("recurse", "recurse(.[]?)", "safe recursive descent (error-suppressed)", InputType::Any)` after the existing bare `recurse` entry

## 3. Add strptime / strftime format entries

- [x] 3.1 Add three additional `strptime` entries after the existing `strptime("%Y-%m-%d")` entry, all with `InputType::String`:
  - `("strptime", "strptime(\"%Y-%m-%dT%H:%M:%S\")", "parse ISO datetime string", InputType::String)`
  - `("strptime", "strptime(\"%d/%m/%Y\")", "parse day/month/year string", InputType::String)`
  - `("strptime", "strptime(\"%H:%M:%S\")", "parse time string", InputType::String)`
- [x] 3.2 Add three additional `strftime` entries after the existing `strftime("%Y-%m-%d")` entry, all with `InputType::Number`:
  - `("strftime", "strftime(\"%Y-%m-%dT%H:%M:%SZ\")", "format as ISO datetime", InputType::Number)`
  - `("strftime", "strftime(\"%H:%M:%S\")", "format as time", InputType::Number)`
  - `("strftime", "strftime(\"%Y/%m/%d %H:%M\")", "format as date and time", InputType::Number)`

## 4. Fix values detail string

- [x] 4.1 Change the `values` detail string from `"values as array"` to `"filter nulls (select(!= null))"` in the `BUILTINS` constant

## 5. Add tests for new catalog entries

- [x] 5.1 Add test `flatten_depth_form_appears_for_array` — assert `get_completions("flatten", Some("array"))` returns both `"flatten"` and `"flatten(1)"` insert texts
- [x] 5.2 Add test `flatten_entries_absent_for_non_array` — assert `get_completions("flatten", Some("string"))` returns no flatten entries
- [x] 5.3 Add test `range_all_three_forms_appear` — assert `get_completions("range", None)` returns insert texts `"range(10)"`, `"range(0; 10)"`, and `"range(0; 10; 2)"`
- [x] 5.4 Add test `paths_predicate_form_appears` — assert `get_completions("paths", None)` returns both `"paths"` and `"paths(scalars)"` insert texts
- [x] 5.5 Add test `recurse_safe_form_appears` — assert `get_completions("recurse", None)` returns both `"recurse"` and `"recurse(.[]?)"` insert texts
- [x] 5.6 Add test `strptime_format_variants_appear_for_string` — assert `get_completions("strptime", Some("string"))` returns insert texts for all four strptime formats including `"%Y-%m-%dT%H:%M:%S"`, `"%d/%m/%Y"`, `"%H:%M:%S"`
- [x] 5.7 Add test `strftime_format_variants_appear_for_number` — assert `get_completions("strftime", Some("number"))` returns insert texts for all four strftime formats including `"%Y-%m-%dT%H:%M:%SZ"`, `"%H:%M:%S"`, `"%Y/%m/%d %H:%M"`
- [x] 5.8 Add test `strptime_absent_for_number_input` — assert `get_completions("strptime", Some("number"))` is empty
- [x] 5.9 Add test `strftime_absent_for_string_input` — assert `get_completions("strftime", Some("string"))` is empty
- [x] 5.10 Add test `values_detail_string_reflects_jaq_semantics` — assert the `values` item detail does not contain `"values as array"` and contains `"null"` or `"select"`

## 6. Run full test suite

- [x] 6.1 Run `cargo test` — all tests must pass with no failures or warnings

## 7. Move resolved maturity files to completed

- [x] 7.1 Move `docs/maturity/flatten.md` → `docs/maturity/completed/flatten.md`
- [x] 7.2 Move `docs/maturity/range.md` → `docs/maturity/completed/range.md`
- [x] 7.3 Move `docs/maturity/paths.md` → `docs/maturity/completed/paths.md`
- [x] 7.4 Move `docs/maturity/recurse.md` → `docs/maturity/completed/recurse.md`
- [x] 7.5 Move `docs/maturity/keys-values.md` → `docs/maturity/completed/keys-values.md`
- [ ] 7.6 Move `docs/maturity/until-while.md` → `docs/maturity/completed/until-while.md` (if the optional entries from task 2 were added; skip otherwise)

## 8. Update docs/maturity/README.md

- [x] 8.1 Remove `flatten`, `range`, `paths`, `recurse`, and `keys-values` from the active tables (Array, Universal, Path, Object sections) and their Low complexity list entries
- [x] 8.2 Update the "32 functions moved to completed" count to reflect the newly moved files
- [x] 8.3 Verify all remaining links in README point to files that exist
