## Why

The completion dropdown currently only surfaces items whose label starts with the typed token. If a user types `up` hoping to find `ascii_upcase`, or `str` hoping to find `tostring`, they get nothing. Fuzzy matching lets users find functions by any memorable fragment — middle of the name, abbreviation, or rough approximation — without needing to remember exact prefixes.

## What Changes

- The completion engine gains a fuzzy-match fallback: when fewer than N exact-prefix matches exist, fuzzy candidates are appended below them
- Fuzzy matching is applied to jq builtin completions and json-context field completions; LSP completions are passed through as-is (the LSP handles its own ranking)
- Exact prefix matches are always shown first; fuzzy-only matches appear below, visually distinguished with a separator or different styling
- Fuzzy scoring is based on subsequence matching (all typed characters appear in order in the label) with a score that rewards contiguous runs and earlier matches
- No new runtime dependencies — implemented directly in the completions module in pure Rust

## Capabilities

### New Capabilities

- `fuzzy-completion-matching`: Subsequence-based fuzzy matching and scoring for completion candidates, with ranked ordering and visual separation from exact matches

### Modified Capabilities

(none — `jq_builtins::get_completions` and `json_context::get_completions` return `Vec<CompletionItem>` and that signature stays the same; fuzzy is layered in `compute_suggestions` in `main.rs`)

## Impact

- `src/completions/mod.rs` or new `src/completions/fuzzy.rs`: fuzzy scoring function
- `src/main.rs` `compute_suggestions()`: after collecting exact matches, run fuzzy pass and append non-duplicate results
- `src/widgets/query_input.rs`: optional visual marker on fuzzy suggestions (e.g. `~` prefix on detail, or dim styling)
- No changes to `CompletionItem` struct, LSP path, or keymap
