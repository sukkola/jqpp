## 1. Data Structures — WizardState

- [x] 1.1 Define `WizardStep` enum in `src/accept.rs`: `Keyword`, `Stream`, `StreamSubArg { idx: usize }`, `BindKeyword`, `VarName`, `Init`, `UpdateAccum`, `UpdateOp`, `Extract`
- [x] 1.2 Define `WizardFrame` struct: `{ step: WizardStep, saved_query: String, saved_cursor: usize, saved_suggestions: Vec<Suggestion> }`
- [x] 1.3 Define `WizardState` struct: `{ keyword: WizardKeyword (Foreach|Reduce), stack: Vec<WizardFrame>, accumulated: String }`
- [x] 1.4 Add `wizard_state: Option<WizardState>` field to `App` struct in `src/app.rs`

## 2. Catalog Entry Changes

- [x] 2.1 Change `foreach` catalog entry `insert_text` to `"foreach"` and `detail` to `"foreach-wizard"` in `src/completions/jq_builtins.rs`
- [x] 2.2 Change `reduce` catalog entry `insert_text` to `"reduce"` and `detail` to `"reduce-wizard"` in `src/completions/jq_builtins.rs`
- [x] 2.3 Add `is_foreach_reduce_wizard_suggestion(detail: Option<&str>) -> bool` helper to `src/accept.rs`
- [x] 2.4 Update `is_builder_suggestion` in `src/accept.rs` to include the new wizard detail values

## 3. Stream Suggestion Generator

- [x] 3.1 Add `stream_suggestions(input_type: Option<&str>) -> Vec<CompletionItem>` to `src/suggestions.rs` returning the ranked stream list (`.[]`, `.`, `to_entries[]`, `range(0; 5)`, etc.)
- [x] 3.2 Implement JSON-context ranking: array input ranks `.[]` first; object input ranks `to_entries[]` higher
- [x] 3.3 Mark `range(0; 5)` and `recurse(.children[])` entries with a new detail flag `"stream-sub-wizard"` so handlers can detect sub-wizard entry

## 4. Variable, Init, and Update Suggestion Generators

- [x] 4.1 Add `varname_suggestions(query_prefix: &str) -> Vec<CompletionItem>` to `src/suggestions.rs`: default names `$x`, `$item`, `$acc` plus any bound vars extracted from context
- [x] 4.2 Add `init_suggestions(input_type: Option<&str>) -> Vec<CompletionItem>` to `src/suggestions.rs`: type-aware ordering (`0` default for most; `{}` for object input)
- [x] 4.3 Add `update_accum_suggestions(var_name: &str, json_input: Option<&serde_json::Value>) -> Vec<CompletionItem>`: offers `.`, `$<var>`, and top-level field paths
- [x] 4.4 Add `update_op_suggestions(accum: &str, var_name: &str) -> Vec<CompletionItem>`: offers `. + $<var>`, `. - $<var>`, `. * $<var>`, `. + 1`, `. - 1`, `$<var>` (replace)
- [x] 4.5 Add `extract_suggestions() -> Vec<CompletionItem>`: offers `)` (default) and `; .`

## 5. Wizard Step Transition Logic

- [x] 5.1 Add `wizard_enter_keyword(keyword: WizardKeyword, query: &str, cursor: usize) -> WizardStepResult` to `src/accept.rs`: inserts keyword + space, returns `Stream` step suggestions
- [x] 5.2 Add `wizard_accept_stream(selected: &str, is_sub_wizard: bool, state: &WizardState, query: &str, cursor: usize) -> WizardStepResult`: handles simple stream (→ BindKeyword) and sub-wizard entry (→ StreamSubArg)
- [x] 5.3 Add `wizard_accept_stream_sub_arg(idx: usize, selected: &str, keyword: WizardKeyword, query: &str, cursor: usize) -> WizardStepResult`: handles range slot 0→1, slot 1→BindKeyword; recurse slot 0→BindKeyword
- [x] 5.4 Add `wizard_accept_bind_keyword(selected: &str, query: &str, cursor: usize) -> WizardStepResult`: `as` → VarName; `|` → exit wizard with pipe
- [x] 5.5 Add `wizard_accept_var_name(selected: &str, query: &str, cursor: usize) -> WizardStepResult`: inserts `$<name> (`, → Init
- [x] 5.6 Add `wizard_accept_init(selected: &str, query: &str, cursor: usize) -> WizardStepResult`: inserts `<init>; `, → UpdateAccum
- [x] 5.7 Add `wizard_accept_update_accum(selected: &str, query: &str, cursor: usize) -> WizardStepResult`: inserts accum prefix, → UpdateOp
- [x] 5.8 Add `wizard_accept_update_op(selected: &str, keyword: WizardKeyword, query: &str, cursor: usize) -> WizardStepResult`: for reduce → closes `)`, exits; for foreach → Extract step
- [x] 5.9 Add `wizard_accept_extract(selected: &str, query: &str, cursor: usize) -> WizardStepResult`: `)` → closes clause; `; .` → inserts `; .)` with cursor at `.`, exits

## 6. Enter Fast-Forward Logic

- [x] 6.1 Add `wizard_fast_forward(keyword: WizardKeyword, current_step: &WizardStep, partial_query: &str, cursor: usize) -> (String, usize)` to `src/accept.rs`: applies defaults for current step and all remaining steps in sequence
- [x] 6.2 Canonical defaults: stream=`.[]`, bind=`as`, var=`$x`, init=`0`, accum=`.`, op=`. + $x`, extract=`)`

## 7. Esc Step-Back Logic

- [x] 7.1 Add `wizard_pop_step(state: &mut WizardState) -> Option<(String, usize, Vec<Suggestion>)>` to `src/accept.rs`: pops top frame and returns saved query/cursor/suggestions; returns `None` if stack empty (exits wizard)

## 8. Handler Wiring

- [x] 8.1 In `src/handlers.rs`, add `is_foreach_reduce_wizard_suggestion` check alongside existing builder checks in the Tab handler branch
- [x] 8.2 Tab handler: if `wizard_state.is_some()`, route to `wizard_accept_<current_step>`; else use entry check (`is_foreach_reduce_wizard_suggestion`) to initialise wizard
- [x] 8.3 Enter handler: if `wizard_state.is_some()`, call `wizard_fast_forward` and clear wizard state
- [x] 8.4 Esc handler: if `wizard_state.is_some()`, call `wizard_pop_step`; if empty stack, clear wizard state and close dropdown as normal
- [x] 8.5 Any non-Tab/Enter/Esc keypress handler: if `wizard_state.is_some()`, clear wizard state before normal key processing

## 9. Suggestion-Activation Integration

- [x] 9.1 Ensure `suggestion_active` flag and `WizardActive` distinction are consistent: when `wizard_state.is_some()`, the `all_exact` guard in `handle_finished_computes` does not prematurely close suggestions
- [x] 9.2 Verify that the debounce path in `run_debounced_compute` does not reset wizard suggestions mid-step

## 10. Unit Tests — Wizard Step Transitions

- [x] 10.1 Test `wizard_enter_keyword` for both `foreach` and `reduce`: verify query and cursor position
- [x] 10.2 Test every step transition (Stream → BindKeyword → VarName → Init → UpdateAccum → UpdateOp → Extract → close) for `foreach`
- [x] 10.3 Test every step transition for `reduce` (no Extract step)
- [x] 10.4 Test range sub-wizard: slot 0 → slot 1 → BindKeyword
- [x] 10.5 Test recurse sub-wizard: slot 0 → BindKeyword
- [x] 10.6 Test `wizard_fast_forward` from each step for both keywords
- [x] 10.7 Test `wizard_pop_step` at every position including boundary (Esc from step 0)
- [x] 10.8 Test `|` bind-keyword selection exits wizard correctly
- [x] 10.9 Test extract step: `)` closes clause; `; .` inserts extract slot and exits

## 11. Unit Tests — Suggestion Content

- [x] 11.1 Test `stream_suggestions` for array input: `.[]` is first
- [x] 11.2 Test `stream_suggestions` for object input: `to_entries[]` ranks higher than `.[]`
- [x] 11.3 Test `varname_suggestions` with no bound vars: default list returned
- [x] 11.4 Test `varname_suggestions` with outer bound vars: bound names appear in list
- [x] 11.5 Test `init_suggestions` for object input: `{}` is first
- [x] 11.6 Test `init_suggestions` for array input: `0` is first
- [x] 11.7 Test `update_op_suggestions` includes `$x` (replace) and arithmetic variants

## 12. Unit Tests — Esc Behavior and Manual Edit Exit

- [x] 12.1 Test that manual character input clears `wizard_state` and normal suggestion logic applies
- [x] 12.2 Test full Esc sequence: Init → Esc → VarName, VarName → Esc → BindKeyword, BindKeyword → Esc → Stream, Stream → Esc → wizard exits
- [x] 12.3 Test that `all_exact` guard does not close wizard suggestions prematurely during a multi-step session
