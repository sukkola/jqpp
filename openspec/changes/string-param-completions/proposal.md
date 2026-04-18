## Why

Functions like `split`, `startswith`, `endswith`, `ltrimstr`, `rtrimstr`, `contains`, and `index` currently suggest a static placeholder (`split(",")`) that ignores what string values actually exist in the input at runtime. A user querying `.orders[].order_status | split("_")` already has all the strings in front of them — the IDE should be able to suggest `_` as a separator candidate, `"shipped"` as a startswith candidate, etc. The completions should be driven by the function's semantics and the live runtime values, not a generic empty quote pair.

## What Changes

- Each jq string-parameter function is mapped to a **string extraction strategy** that determines what kind of candidates to extract from the runtime string values:
  - `startswith`, `ltrimstr` → unique **prefixes** (leading tokens / character runs)
  - `endswith`, `rtrimstr` → unique **suffixes** (trailing tokens / character runs)
  - `split`, `contains`, `index`, `rindex`, `indices` → recurring **internal substrings** (separator and pattern candidates)
- The insert-text for these functions in the builtin catalog changes from `fn("placeholder")` to `fn()` (empty parens, cursor placed inside) so that parameter completions fire immediately on Tab-accept
- A new `string_param_context` detector (parallel to the existing `param_field_context`) recognises when the cursor is inside a string-param function call and extracts the inner prefix typed so far
- A new `string_param_completions` helper collects runtime string values from the JSON context, runs the appropriate extraction strategy, and returns sorted `CompletionItem`s whose `insert_text` wraps the value in quotes: `split(",")`, `startswith("ship")`, etc.
- The data structure for candidate retrieval is a **sorted `Vec<String>`** with binary-search for prefix filtering and a linear scan with subsequence scoring for fuzzy matching — no radix tree, consistent with the existing fuzzy pipeline
- Functions with regex or multi-part arguments (`test`, `match`, `scan`, `sub`, `gsub`, `capture`, `strptime`, `strftime`) are **explicitly excluded**; they take patterns, not literal values
- `@tsv` and `@csv` format operators are restricted to **arrays of scalars** to prevent runtime errors
- JSON context suggestions now **evaluate the query prefix before pipes**, ensuring accurate field names even after complex transformations

## Capabilities

### New Capabilities

- `string-param-completions`: Context-aware string value completions inside the argument parens of string-literal-parameter functions, with per-function extraction strategies and sorted-Vec retrieval

### Modified Capabilities

- `completions`: The builtin catalog insert-texts for `split`, `startswith`, `endswith`, `ltrimstr`, `rtrimstr`, `contains`, `index`, `rindex`, `indices` change from placeholder-quoted strings to empty-parens, altering Tab-accept UX for these functions

## Impact

- `src/completions/json_context.rs` — `string_param_context`, `StringParamCtx`, extraction helpers, `string_param_completions`, called from `get_completions`
- `src/completions/jq_builtins.rs` — insert-text changes for 9 functions
- No changes to `main.rs`, widget layer, or async pipeline
- No new external dependencies
