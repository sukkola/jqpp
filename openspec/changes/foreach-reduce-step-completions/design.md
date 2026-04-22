## Context

`foreach` and `reduce` are jq's most complex built-in expressions. Each has five or more distinct syntactic slots (`<stream>`, binding keyword, `$var`, `<init>`, `<update>`, and optionally for `foreach` an `<extract>`). The current completion inserts a complete but fixed template, giving the user no per-slot guidance.

The existing "builder" pattern (used for `contains` and numeric args) already proves that multi-step Tab/Enter/Esc flows work well for multi-part expressions. This design extends that pattern to handle the far richer structure of `foreach`/`reduce`.

The app already has a `WizardState`-like concept implicitly (the `suggestion_active` bool + `detail`-based dispatch in handlers). We formalise it as an explicit stack-based state machine.

## Goals / Non-Goals

**Goals:**

- Replace single-shot `foreach`/`reduce` template insertion with a slot-by-slot wizard.
- Each Tab advances exactly one slot; each Esc walks back one slot via a state stack.
- Enter at any step fast-forwards through all remaining slots using canonical defaults, then places cursor after the closing `)`.
- Stream sub-wizards for `range(N; M)` and `recurse(expr)` (the two parametric streams with their own interior slots).
- Context-aware suggestions at each slot (init uses JSON type of current input; update operands use bound `$var`; stream uses JSON structure).
- Optional extract-clause step for `foreach` (Tab-selectable `; .` vs `)` default).
- Thorough unit tests: every slot transition, Esc stack pop, Enter fast-forward, and sub-wizard path.

**Non-Goals:**

- Wizard for any other multi-arg expression beyond `foreach`/`reduce` (future work).
- Full jq expression parsing inside wizard slots (we use pattern-based heuristics, same as the rest of the completion system).
- GUI overlays beyond the existing suggestion dropdown (no extra UI widgets).

## Decisions

### D1 — Explicit `WizardState` stack on `App`

**Decision**: Add `wizard_state: Option<WizardState>` to `App`, where `WizardState` holds a `Vec<WizardStep>` stack plus accumulated query string.

**Alternatives considered**:
- *Reuse existing `detail` string dispatch*: Would require encoding step index in the `detail` field of every suggestion item. Fragile and not type-safe.
- *Implicit parsing of query on every keypress*: Possible but slow, re-parses the growing query on each Tab; the explicit stack is O(1).

**Rationale**: The stack makes Esc trivially correct — pop the top frame, restore its saved query and suggestion set. The alternative approaches would make Esc reconstruction complex.

---

### D2 — Wizard entry via `detail = "foreach-wizard"` / `"reduce-wizard"`

**Decision**: The `foreach` and `reduce` catalog entries (in `jq_builtins.rs`) change their `insert_text` to just the keyword (`"foreach"` / `"reduce"`) and their `detail` to `"foreach-wizard"` / `"reduce-wizard"`. Accepting such a completion triggers wizard mode instead of inserting a template.

**Alternatives considered**:
- *Keyword-only catalog entries with a separate lookup table*: More indirection for no benefit.
- *Detect by matching `insert_text` against regex*: Fragile.

**Rationale**: The `detail` field is already the dispatch mechanism for all builder types (`"flatten nested arrays"`, `"integer generator"`, `"contains object key"`, etc.). Using the same pattern keeps `handlers.rs` consistent.

---

### D3 — Wizard step enum with typed slot data

**Decision**: Define `WizardStep` as an enum:
```
Keyword               → just accepted "foreach"/"reduce"
Stream                → selecting stream expression
StreamSubArg { idx }  → inside range/recurse sub-wizard, slot index
BindKeyword           → choosing "as" vs "|"
VarName               → choosing $var
Init                  → choosing init expression
UpdateAccum           → choosing accumulator prefix (`.`, `$x`, field)
UpdateOp              → choosing operation (`. + $x`, `. - $x`, ...)
Extract               → choosing `)` vs `; <extract>` (foreach only)
```

Each step knows its canonical default so Enter can fast-forward without further logic.

---

### D4 — Stream suggestion catalog

**Decision**: Offer a fixed ordered list of stream options at the Stream step:

| Priority | Label | Insert text | Sub-wizard? |
|---|---|---|---|
| 1 | `.[]` | `.[]` | No |
| 2 | `.` | `.` | No |
| 3 | `to_entries[]` | `to_entries[]` | No |
| 4 | `range(0; 5)` | `range(0; 5)` | Yes — 2 slots |
| 5 | `.[] \| select(. > 0)` | `.[] \| select(. > 0)` | No (edit manually) |
| 6 | `recurse(.children[])` | `recurse(.children[])` | Yes — 1 slot |
| 7 | `(.a[], .b[])` | `(.a[], .b[])` | No |
| 8 | `inputs` | `inputs` | No |
| 9 | `paths(scalars)` | `paths(scalars)` | No |

JSON-context filtering: if input is an array, `.[]` is ranked first; if input is an object, `to_entries[]` is ranked higher.

---

### D5 — Sub-wizard for `range` and `recurse`

**Decision**: `range` uses a 2-slot sub-wizard (`start`, `end`); the step flag is triggered by selecting the `range(0; 5)` option. The query is written as `foreach range(<cursor>; 5) as …` then `foreach range(0; <cursor>) as …`. `recurse` uses a 1-slot sub-wizard (`expr`). After the sub-wizard completes, the main wizard continues to `BindKeyword`.

---

### D6 — Init suggestions are JSON-type-aware

**Decision**: At the Init step, suggestions are derived from the current executor JSON input type:
- Any type: `0`, `null`
- Array input: `[]`, `0`
- Object input: `{}`, `null`
- String input: `""`, `""`
The first suggestion is always `0` for numeric accumulation (most common use case).

---

### D7 — Update expression: two sub-steps

**Decision**: Two sub-steps within the update slot:
1. **UpdateAccum** — suggests `.` (default), `$<var>`, and top-level field paths from JSON context. Accepting inserts the token into the update expression and advances to UpdateOp.
2. **UpdateOp** — suggests operations relative to the chosen accumulator. Default: `+ $<var>`. Full list: `+ $<var>`, `- $<var>`, `* $<var>`, `+ 1`, `- 1`, `$<var>` (replace). Accepts one and closes the update expression.

---

### D8 — Extract step offer for `foreach`

**Decision**: After UpdateOp, `foreach` shows an extra step with two options:
- `)` — close with 2-arg form (default, Enter fast-forwards here)
- `; .` — add extract clause (cursor moves inside `;` for the user to customise)

`reduce` skips this step entirely.

---

### D9 — Esc pops state stack

**Decision**: Pressing Esc during wizard mode pops the top `WizardStep`, restores the saved query string and suggestion list from the frame below. If the stack becomes empty (Esc from the very first Keyword step), wizard mode exits and normal suggestion close behaviour applies.

---

### D10 — Enter fast-forwards with canonical defaults

**Decision**: At any step, Enter applies the canonical default for the current slot and all subsequent slots in order, producing the minimal-but-valid form. For `foreach`: `.[] as $x (0; . + $x)`. For `reduce`: `.[] as $x (0; . + $x)`. Cursor lands after `)`.

## Risks / Trade-offs

- **Complexity spike**: This is the most stateful interaction logic added so far. Mitigation: strict unit tests for every transition; the `WizardState` struct is self-contained and not entangled with the rest of the app.
- **Sub-wizard nesting depth**: `range` inside `foreach` inside a longer pipeline is valid jq. The wizard only covers the first level (the immediately typed `foreach`/`reduce`); any outer context is already committed text and not re-parsed. Mitigation: document this as a non-goal.
- **Cursor position bugs**: Multi-step cursor movement across a growing string is error-prone. Mitigation: each step function is a pure function of `(current_query, cursor_col, selection)` → `(new_query, new_col, new_step)`; unit-tested exhaustively.
- **Esc stack memory**: A deeply nested wizard session could accumulate many frames. In practice this is bounded by the number of wizard steps (max ~8). No significant memory concern.

## Migration Plan

1. Change `foreach` / `reduce` catalog entries (non-breaking for normal users; the keyword still appears in the dropdown).
2. Add `WizardState` to `App` (additive; no existing fields removed).
3. Add wizard dispatch in `handlers.rs` behind the `detail == "foreach-wizard"` / `"reduce-wizard"` check (no change to non-wizard paths).
4. Add `apply_foreach_reduce_wizard_step` and helpers to `accept.rs`.
5. Add stream/variable/init/update suggestion generators to `suggestions.rs`.
6. Write tests at each stage.
