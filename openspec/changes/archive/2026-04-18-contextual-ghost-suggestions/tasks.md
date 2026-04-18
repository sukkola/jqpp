## 1. App State

- [x] 1.1 Add `structural_hint_active: bool` field to `App` (default `false`) to distinguish passive structural ghosts from user-initiated completions
- [x] 1.2 Add `dismissed_hint_query: Option<String>` field to `App` (default `None`) to suppress re-showing a hint the user explicitly dismissed with Esc

## 2. Structural Hint Inference

- [x] 2.1 Add `pub fn next_structural_hint(query_prefix: &str, input: &Value) -> Option<Vec<CompletionItem>>` to `src/completions/json_context.rs`:
  - If query ends with `[]` and the first array element is an object → return `[CompletionItem { label: ".", insert_text: query + ".", detail: None }]`
  - Else if the value at query path is an array and query doesn't end with `[` → return `[CompletionItem { label: "[]", insert_text: query + "[]", detail: None }]`
  - Otherwise return `None`
- [x] 2.2 Add unit tests for `next_structural_hint`: array path returns `[]`, array-of-objects path returns `.`, scalar path returns `None`, query ending with `[` returns `None`

## 3. Wire Hints Into Main Loop

- [x] 3.1 In `main.rs`, after `compute_handle` resolves (results or error arrive) and `!suggestion_active`:
  - If `app.dismissed_hint_query.as_deref() == Some(current_query_prefix)`, skip hint
  - Otherwise call `next_structural_hint` and if `Some(items)` returned: set `app.query_input.suggestions = items_as_suggestion_vec`, `app.query_input.show_suggestions = true`, `app.structural_hint_active = true`
- [x] 3.2 On every keystroke that modifies the query: if `app.dismissed_hint_query` is set and the new query differs from it, clear `app.dismissed_hint_query`
- [x] 3.3 When `suggestion_active` becomes true (user-initiated completion), clear `app.structural_hint_active` and do not overwrite `show_suggestions` from structural hints

## 4. Esc Handling

- [x] 4.1 In the Esc handler in `main.rs`, add a branch: if `app.structural_hint_active`, dismiss the hint — set `structural_hint_active = false`, `show_suggestions = false`, clear `suggestions`, set `dismissed_hint_query = Some(current_query_prefix.clone())`. Do NOT clear the query text.
- [x] 4.2 Ensure the existing double-Esc (clear query) path is checked AFTER the structural hint Esc path so a single Esc only clears the ghost

## 5. Tab Handling

- [x] 5.1 When Tab is pressed and `structural_hint_active = true`, accept the hint: insert `suggestions[0].insert_text` into the query bar, clear `structural_hint_active`, clear `show_suggestions`
- [x] 5.2 If the hint was `.` (object descent), after inserting trigger a normal suggestion cycle so field completions appear immediately

## 6. Up/Down Handling

- [x] 6.1 When Up or Down is pressed and `structural_hint_active = true`, transition to full suggestion mode: call `compute_suggestions` with `query_prefix + hint_char` as the prefix, set `suggestion_active = true`, set `structural_hint_active = false`, open dropdown normally

## 7. Tests

- [x] 7.1 Unit test in `src/completions/json_context.rs`: `next_structural_hint` returns `[]` for array value, `.` for array-of-objects, `None` for string
- [x] 7.2 Unit/integration test: structural hint is suppressed when `dismissed_hint_query` matches current prefix
- [x] 7.3 Unit test: structural hint is cleared and `dismissed_hint_query` set when Esc pressed on active ghost
