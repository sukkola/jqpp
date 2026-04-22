## Why

`foreach` and `reduce` are the most structurally complex jq expressions — five or more distinct syntactic slots — yet the current completion inserts a single fixed template (`foreach .[] as $x (0; . + $x)`) that gives the user no guidance and forces them to manually overwrite every part. A step-by-step wizard that walks each slot in sequence, with context-aware suggestions at each step, dramatically lowers the learning curve and prevents syntax errors.

## What Changes

- **Remove** the single-shot template insertion for `foreach` and `reduce` completions.
- **Add** a multi-step wizard mode that activates when either keyword is accepted; each Tab advances one slot, each Esc steps back one slot, and Enter at any point fills sensible defaults for all remaining slots and closes the clause.
- **Add** a wizard state stack to the app layer so Esc can restore the previous slot's suggestion set.
- **Add** stream sub-wizards for parametric streams (`range`, `recurse`) so their arguments can also be filled interactively before advancing to `as $var`.
- **Add** an optional extract-clause step for `foreach` (the 3rd semicolon argument), offered after the update step.

## Capabilities

### New Capabilities

- `foreach-reduce-wizard`: Step-by-step interactive builder for `foreach` and `reduce` clauses, replacing single-shot template insertion. Covers stream selection (with sub-wizards for parametric streams), binding keyword, variable naming, init expression, two-phase update expression, optional foreach extract clause, and full Esc-to-previous-step navigation via a state stack.

### Modified Capabilities

- `suggestion-activation`: Wizard mode introduces a new `WizardActive` suggestion-active sub-state that must be distinguished from normal `SuggestionActive` so that Tab/Enter/Esc are routed through wizard logic instead of standard completion logic.

## Impact

- `src/accept.rs` — new `apply_foreach_reduce_wizard_step` function and supporting helpers; wizard step enum.
- `src/handlers.rs` — Tab/Enter/Esc branches extended to detect and dispatch wizard state.
- `src/app.rs` — `App` struct gains a `wizard_state: Option<WizardState>` field with a step stack.
- `src/completions/jq_builtins.rs` — `foreach` and `reduce` catalog entries changed from full-template `insert_text` to keyword-only, flagged as wizard entry points via a new `detail` value.
- `src/suggestions.rs` — stream, variable, init, and update suggestion generators.
- Tests: new integration-style unit tests for every wizard step transition, Esc stack behavior, and Enter-to-default fast-path.
