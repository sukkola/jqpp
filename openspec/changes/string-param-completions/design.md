## Context

The existing `param_field_completions` (from `param-field-completions` change) provides the pattern: detect cursor inside a known function's parens, resolve the JSON context, extract candidates, return `CompletionItem`s. This change adds a parallel system for functions whose parameter is a quoted string literal drawn from the runtime string values.

The runtime string values reachable at a path are always a flat, dynamic set. Per the design research in the user request:
- For small/medium dynamic string sets a **sorted `Vec<String>`** with binary search is the right structure — simpler and faster than a radix/trie due to cache locality
- Fuzzy candidates come from a linear scan with subsequence scoring (same scorer as `src/completions/fuzzy.rs`)
- Radix trees, BK-trees, n-gram indices are not needed at this scale; jq inputs are typically tens to low thousands of string values

### Function-to-strategy mapping

| Function | Strategy | Rationale |
|---|---|---|
| `startswith(s)` | Leading **prefixes** | s must match the start of the string |
| `ltrimstr(s)` | Leading **prefixes** | strips the leading prefix |
| `endswith(s)` | Trailing **suffixes** | s must match the end of the string |
| `rtrimstr(s)` | Trailing **suffixes** | strips the trailing suffix |
| `split(s)` | **Internal** recurring substrings | s is a separator; likely appears repeatedly |
| `contains(s)` | **Internal** substrings | s can be anywhere inside |
| `index(s)` | **Internal** substrings | finds first occurrence |
| `rindex(s)` | **Internal** substrings | finds last occurrence |
| `indices(s)` | **Internal** substrings | finds all occurrences |

Regex / format-string functions (`test`, `match`, `scan`, `sub`, `gsub`, `capture`, `strptime`, `strftime`) are excluded — their parameters are not drawn from the string values.

### Prefix strategy

For a set of strings, extract prefixes by tokenising on common delimiter chars (`-`, `_`, `/`, `.`, `@`, ` `, `\t`, `,`, `|`) and collecting the leading token. Also include the full string itself. Deduplicate and order exact matches shortest-first.

Example: `["CUST-42", "CUST-17", "CUST-09"]` → prefix candidates `["CUST", "CUST-"]`

### Suffix strategy

Generate layered right-to-left suffix candidates so Tab can extend meaningfully: final token (`com`), token-chain (`corp.com`), delimiter+token-chain (`@corp.com`), and full string.

Example: `["alice@example.com", "mikko@example.com"]` → suffix candidates `["com", "example.com", "@example.com"]`

### Internal (separator/substring) strategy

For separator functions like `split`, the relevant candidates are substrings that appear in multiple strings. Approach:
1. Collect single-char delimiter candidates (any non-alphanumeric character that appears in ≥ 2 strings)
2. Collect multi-char substrings by splitting on alphanumeric runs and taking the delimiters between them
3. Deduplicate and sort

For `contains`/`index`/`rindex`/`indices`, the candidates are the full strings themselves (the most common use is checking if a known value appears anywhere).

This distinction is captured by a `StringParamStrategy` enum.

## Goals / Non-Goals

**Goals**
- Completions inside the argument of the 9 target functions
- Candidates sourced from runtime string values in the JSON input at the context path
- Per-function extraction strategy (prefix / suffix / internal)
- Sorted-Vec retrieval with binary search for prefix filtering, linear scan for fuzzy
- Insert-text wraps selected value in double quotes: `split(",")`, `startswith("CUST-")`
- Tab UX: accepting function from dropdown leaves cursor inside empty parens, completions fire automatically

**Non-Goals**
- Regex-aware suggestions inside `test`, `match`, `scan`, `sub`, `gsub`
- Format-string suggestions inside `strptime`, `strftime`
- Multi-segment argument completions (e.g., second arg of `sub`)
- Persistent/memoized candidate index across keystrokes (rebuilt on each call; cheap for typical jq inputs)
- Suggesting beyond the string values visible in the current JSON input (no frequency history)

## Decisions

### D1: Detector placement — json_context.rs

Add `pub fn string_param_context(query: &str) -> Option<StringParamCtx>` and `fn string_param_completions(query, input, out)` in `json_context.rs`, called from `get_completions`. Same placement as `param_field_completions`. No changes to `main.rs`.

### D2: StringParamCtx structure

```rust
pub struct StringParamCtx<'a> {
    pub fn_name: &'a str,        // "split", "startswith", …
    pub strategy: StringParamStrategy,
    pub context_path: &'a str,   // path that feeds into the function
    pub inner_prefix: &'a str,   // what user typed inside the parens (may be empty or quoted)
}

pub enum StringParamStrategy { Prefix, Suffix, Internal, FullString }
```

`FullString` is used for `contains`/`index`/`rindex`/`indices` where the full string value is the most useful candidate.

### D3: inner_prefix parsing — strip quotes

The inner content may be bare (`split(,`) or quote-started (`split(","` when cursor is between the quotes). The detector normalises both to the bare typed content. Specifically:
- If `inner_raw` starts with `"`, strip the leading quote to get `inner_prefix`
- The closing `"` is never present in `query_prefix` (cursor is before it)

### D4: Collecting string values from JSON

Walk the resolved value at `context_path`. If it is:
- A `String` → single-element set
- An `Array` → collect all `String` elements, skip non-strings
- An `Object` → collect all `String` values, skip non-strings
- Otherwise → empty set, return no completions

This is synchronous, purely a tree walk, O(n) in value count.

### D5: Insert-text construction

`insert_text = format!("{}\"{}\"", query_up_to_open_paren, candidate)`

Where `query_up_to_open_paren` is everything before and including the `(`. This replaces the whole `query_prefix` so Tab gives a correct, complete expression.

### D6: Builtin insert-text changes

Change the 9 affected functions from `fn("placeholder")` to `fn()`. This means:
- Tab-accepting the builtin leaves `fn()` with cursor inside the parens
- `string_param_context` immediately fires and populates suggestions
- Existing `cursor_col_after_accept` needs updating: currently moves cursor after `("` — for empty-parens functions, cursor should land at position `fn(` + 1

### D7: Data structure — sorted Vec, no tree

Candidates are built fresh for each invocation (cheap: string extraction is O(n × avg_len)). Exact matches are ordered shortest-first (then lexicographic), fuzzy matches are appended after exact matches and marked with `~` detail. Filtering is strategy-aware: suffix functions match from the end (`ends_with`), other strategies match from the start (`starts_with`).

### D8: Tab and Enter semantics in string-param contexts

- `Tab` extends the current argument toward the next meaningful boundary:
  - Prefix-like functions: extend forward to next token boundary.
  - Suffix-like functions: extend to the next longer suffix that still ends with current input.
- `Enter` commits the currently typed value as a valid call by closing and quoting the argument.

This matches the established pattern in the codebase and is the optimal choice for typical jq input sizes.

## Risks / Trade-offs

- [Large string sets] If a JSON array has tens of thousands of strings, extraction + sort on every keystroke could add latency. Mitigation: cap collection at 500 source strings; this covers all realistic jq inputs.
- [Quote-in-string values] Candidate strings containing `"` would corrupt the insert-text. Mitigation: escape `"` as `\"` in insert-text.
- [False candidates for split] Single-char delimiters can be too aggressive (e.g., every word boundary). Mitigation: only suggest single-char delimiters that appear in ≥ 2 source strings; prefer multi-char runs.
- [cursor_col_after_accept regression] Changing insert-texts from `fn("")` to `fn()` changes the cursor landing position. Mitigation: update `cursor_col_after_accept` to handle the empty-parens pattern too.

## Migration Plan

No external API changes. The only visible change to existing users is the Tab-accept UX for the 9 affected functions, which now requires one more step (select from the string completion box) rather than producing a static placeholder. This is a deliberate UX improvement.
