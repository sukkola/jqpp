## Context

`compute_suggestions()` in `src/main.rs` calls three completion sources and merges them:
1. `json_context::get_completions` — field names from the live JSON input
2. `jq_builtins::get_completions` — ~90 hardcoded builtins, filtered by `starts_with(token)`
3. LSP completions — already ranked by the language server

Both #1 and #2 filter on prefix only (`name.starts_with(token)`). The token is the word the cursor is currently on (extracted by `current_token()`). If the token doesn't appear at the start of any label the dropdown is empty or sparse.

`CompletionItem` has `label`, `insert_text`, and `detail: Option<String>`. The `Suggestion` type in `query_input.rs` wraps this.

## Goals / Non-Goals

**Goals:**
- Subsequence fuzzy matching: all characters of the token appear in the label in order (e.g. `upcase` matches `ascii_upcase`)
- Fuzzy candidates appear only when they don't duplicate an existing exact match
- Fuzzy results are sorted by score (higher = better match); exact prefix matches keep their current ordering above all fuzzy results
- Fuzzy matches are visually marked so users can tell them apart from exact matches
- Applied to builtins and json-context fields; LSP items are passed through unchanged

**Non-Goals:**
- Typo correction / edit-distance (Levenshtein) — subsequence is sufficient and cheaper
- Fuzzy matching on LSP completions — the LSP ranks its own results
- Configurable fuzzy threshold or toggle — always on as a fallback

## Decisions

### D1: Scoring algorithm — subsequence with contiguity bonus

Score = number of matched characters (always = token length for a valid subsequence match) + bonus for contiguous runs. Specifically: for each consecutive pair of matched positions that are adjacent in the label, add +2. An earlier first-match position gets a small bonus (+1 per position from the end). This is O(token_len × label_len), fast enough for ~100 candidates.

Alternative: bitap / Levenshtein. Rejected — more complex, handles typos the user didn't ask for.

### D2: Where fuzzy lives — new `src/completions/fuzzy.rs`

A standalone module with two public functions:
- `fuzzy_score(token: &str, label: &str) -> Option<i32>` — returns `None` if not a subsequence match
- `fuzzy_completions(token: &str, items: &[CompletionItem]) -> Vec<(i32, CompletionItem)>` — returns scored matches, sorted descending

`compute_suggestions()` calls this after collecting exact matches, filters out duplicates by label, strips scores, and appends to the merged list.

Alternative: inline in `compute_suggestions`. Rejected — harder to unit test.

### D3: Visual distinction — `~` prefix on detail

Fuzzy-matched items get their `detail` field prepended with `~` (e.g. `~string fn`). This is rendered in the dropdown's secondary column without requiring any structural change to `CompletionItem` or the widget. Users see at a glance which suggestions are fuzzy.

Alternative: a separate `is_fuzzy: bool` field on `CompletionItem` / `Suggestion`. Rejected — requires touching the struct and all construction sites; the `~` prefix achieves the same goal for free.

### D4: Fuzzy only fires when token is non-empty

If the token is empty (cursor after space, `.`, `|`) the full list is shown via the existing exact-match path. Fuzzy would add noise with 0-char tokens. Guard: `if token.is_empty() { return }` in the fuzzy call site.

## Risks / Trade-offs

- [Dropdown grows longer] Fuzzy adds up to ~90 extra items for builtins. Mitigated by the existing dropdown height cap and scroll in `QueryInput`. No user-visible performance concern.
- [False positives] Short tokens (1-2 chars) match almost everything. Mitigated by keeping exact matches first and fuzzy appended — user sees the most relevant results immediately.
- [Detail field collision] If a builtin already has a `~` in its detail, the prefix looks odd. In practice no current builtin detail starts with `~`.
