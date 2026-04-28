## Context

`select()` is jq's primary filter primitive. Currently, the intellisense engine has no special handling for the argument position inside `select(...)`. The `param_field_context` function in `completions/json_context.rs` only recognises functions in `FIELD_PATH_ARRAY_FNS` and `FIELD_PATH_INPUT_FNS`; `select` is absent from both lists (line 117). As a result:

- Typing `select(` offers no suggestions — the cursor lands in an empty argument position with no guidance.
- Tab after `select(` jumps past `)` rather than entering the argument.
- There is no default or fallback condition starter tied to the actual input type.

The suggestion pipeline in `suggestions.rs` already has prior art for similar cases: `in_string_param_context` routes string-param functions to a dedicated completion branch, and `foreach_reduce_stream_expr` routes `foreach`/`reduce` to their own branch. `select` needs an analogous branch.

## Goals / Non-Goals

**Goals:**
- Activate intellisense when the cursor is inside `select(` — offer condition starter suggestions derived from the inferred input type
- The condition starters are partial expressions (e.g. `. > `, `. == `, `length > `) that the user completes — not canned example values
- For object inputs, include field-qualified condition starters using keys visible in the actual JSON (e.g. `.age > `, `.name == `)
- The Tab key enters `select(` instead of jumping past it; once inside, Tab accepts the selected condition starter

**Non-Goals:**
- Full expression parsing or type-checking of the condition body (jq handles that)
- Nested `select()` inside the condition argument (treat the inner content as the same context)
- Wizard-style step-by-step guided entry (condition is free-form after the starter)

## Decisions

### 1. New `select_condition_context` detector in `json_context.rs`

Add a function analogous to `parse_string_param_context` that detects when the cursor is inside `select(`. Return a struct carrying:
- `context_path`: the pipe-chain prefix before `select(` (used to resolve the input value)
- `inner_prefix`: what the user has typed so far inside the parens (for filtering suggestions)

This keeps the detection logic co-located with `param_field_context` and `parse_string_param_context`.

**Alternative considered:** Adding `"select"` to `FIELD_PATH_INPUT_FNS`. Rejected — that list drives field-path (`.field`) completions, which is only one of several condition types `select` accepts.

### 2. Type-driven condition starter generation

Based on the inferred type of the value at `context_path`, generate a small set of condition starters:

| Input type | Condition starters offered |
|---|---|
| number | `. > `, `. < `, `. == `, `. >= `, `. <= `, `. != `, `. % 2 == 0` |
| string | `length > `, `startswith(`, `endswith(`, `test(`, `. == `, `. != null` |
| object | `.field > `, `.field == `, `.field != null`, `has(`, `type == ` per key type |
| array | `length > `, `length == `, `type == "array"` |
| boolean | `. == true`, `. == false`, `.`, `. == null` |
| null | `. == null`, `. != null` |
| mixed/unknown | `. != null`, `type == `, `. == ` |

For object inputs, the object's top-level keys are used to emit `.key op ` starters rather than generic `. op ` ones. The operator is chosen by the key's value type.

**Alternative considered:** Hard-coded suggestion strings independent of input. Rejected per the user's explicit requirement: suggestions must be contextual, not example-lifted.

### 3. New branch in `compute_suggestions` (suggestions.rs)

After the `in_string_param_context` branch and before the `foreach_reduce_stream_expr` branch, add:

```
if in_select_condition_context:
    resolve input value at context_path
    generate condition starters from type
    filter by inner_prefix
    return as Suggestion list
```

This mirrors the existing pattern and avoids touching the general completion path.

### 4. Keep-active behaviour for suggestion-activation

The `suggestion-activation` spec's "Function acceptance starts from empty argument position" requirement already says Tab places cursor inside `()` with follow-up suggestions. The key fix is that `select` must be covered by the same keep-active logic that already keeps suggestions open for `has()` and `contains()`.

No changes to the activation spec are needed beyond tagging `select` as a function that triggers inner-context suggestions. The existing `keep_suggestions_active` logic in `handlers.rs` / `loop_state.rs` must recognise `select(` as an active-trigger site.

## Risks / Trade-offs

- **Condition starters are opinionated**: Not every operator will match what the user wants. Risk is low — these are starters, not final expressions.  
  → Mitigation: Offer the most common operators first; user can ignore and type freely.

- **Pipe-chain evaluation may fail**: If `context_path` evaluation errors (e.g. `.[] | select(` with array input), fall back to type-inferring from the raw input rather than returning no suggestions.  
  → Mitigation: Wrap evaluation in the same `unwrap_or_else` fallback already used in `compute_suggestions`.

- **inner_prefix filtering**: User may type a partial operator (e.g. `. >`). Fuzzy prefix matching on the condition starter label handles this adequately.

## Open Questions

- Should condition starters for object keys be truncated if there are many keys (e.g. >10)? Suggest yes: cap at the most common operators per key rather than listing every key × every operator.
