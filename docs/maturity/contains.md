# `contains`

**Category:** String / Array / Object  
**Input type:** String, Array, or Object

## What's implemented

Three type-adaptive catalog entries:
- `InputType::String` → insert text `contains("")`, detail "string contains substring"
- `InputType::Array` → insert text `contains([])`, detail "array contains all elements from RHS subset"
- `InputType::Object` → insert text `contains({})`, detail "object contains RHS as partial deep match"

### Accept / cursor behavior

**`contains("")` (string form)**:
- Registered in `STRING_PARAM_FULLSTRING_FNS` → context-aware
- **Enter / Tab**: inserts `contains("")` with cursor at position 10 — after `contains("`, before the closing `"` — so the user can immediately type a substring
- `keep_active = true`; string values from the live JSON data are suggested as substring candidates via the `FullString` strategy
- **Esc**: `commit_current_string_param_input` closes the string with the currently typed prefix as the literal value

**`contains([])` (array form)**:
- Context-aware via the `strip_suffix(')')` branch in `starts_context_aware_function_call`
- **Enter / Tab**: inserts `contains([])` with cursor at position 9 — after `contains(`, before `[` — so the user can start building the subset array
- `keep_active = true`; the contains-builder context (`is_contains_builder_suggestion`) generates array-value candidates from live data, allowing interactive subset construction
- **Tab** on a value candidate: appends the value and a `, ` separator, keeps builder open
- **Enter** on a value candidate: appends the value, closes `]`, moves cursor past `)`
- **Esc**: `finalize_contains_builder_on_escape` closes the partial array and moves cursor past `)`

**`contains({})` (object form)**:
- Same context-awareness as the array form
- **Enter / Tab**: inserts `contains({})` with cursor at position 9 — before `{`
- The contains-builder context generates object key/value candidates from live data
- **Tab** on a key: inserts `{key: ` and keeps builder open for a value
- **Tab** on a value: appends `, ` and keeps builder open for more pairs
- **Enter** on a value: closes `}`, moves cursor past `)`
- **Esc**: `finalize_contains_builder_on_escape` closes the partial object and moves cursor past `)`

## What's missing

**Type-gating for string-param completions**: Currently `contains("` triggers string-value suggestions regardless of input type. When the input is an array or object, `contains("` is likely a user mistake — no string-value suggestions should appear. The fix (gating `string_param_context` on runtime input type) is in the open spec (task 3.1–3.2). The gate already exists in `gate_string_param_context` but only for `Some("array" | "array_scalars" | "object")` — when `input_type` is `None` it still returns completions.

**Array and object argument completions depth**: The contains builder handles flat array values and flat object key/value pairs. Nested subset structures (e.g. `contains([{"key": "val"}])`) require generating sub-structures from the live JSON, which is significantly more complex.

## Estimated complexity

`Low` — type-gating the existing string-param completions.  
`High` — nested array/object argument completions (generating subset structures from live data).

## Needs changes to overall logic

Yes (type-gating) — `string_param_context` needs to receive the runtime input type; small change to `suggestions.rs` or `json_context.rs`.  
Yes (nested completions) — would require a new completion strategy with no existing analogue.
