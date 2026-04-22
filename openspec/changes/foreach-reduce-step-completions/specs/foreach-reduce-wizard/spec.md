# foreach-reduce-wizard Specification

## Purpose

Define the interactive step-by-step wizard that guides users through constructing `foreach` and `reduce` expressions one syntactic slot at a time, replacing the previous single-shot template insertion.

---

## ADDED Requirements

### Requirement: Accepting `foreach` or `reduce` enters wizard mode

When the user accepts a `foreach` or `reduce` completion from the dropdown, the system SHALL enter wizard mode rather than inserting a fixed template. Wizard mode inserts the keyword, places the cursor after a space, and immediately shows a suggestion dropdown for the first slot (stream selection).

#### Scenario: foreach completion triggers wizard, not template

- **WHEN** the user accepts the `foreach` completion (Tab or Enter)
- **THEN** the query becomes `foreach ` (keyword + space)
- **AND** the suggestion dropdown shows stream options immediately

#### Scenario: reduce completion triggers wizard, not template

- **WHEN** the user accepts the `reduce` completion
- **THEN** the query becomes `reduce ` (keyword + space)
- **AND** the suggestion dropdown shows stream options immediately

---

### Requirement: Stream step offers ordered, JSON-context-aware stream suggestions

At the stream step the system SHALL present a ranked list of stream expressions. The ranking SHALL reflect the current input type: array input ranks `.[]` first; object input ranks `to_entries[]` higher.

#### Scenario: Default stream list for array input

- **WHEN** the JSON input is an array and the wizard is at the stream step
- **THEN** the suggestion list order is: `.[]`, `.`, `to_entries[]`, `range(0; 5)`, `.[] | select(. > 0)`, `recurse(.children[])`, `(.a[], .b[])`, `inputs`, `paths(scalars)`

#### Scenario: Default stream list for object input

- **WHEN** the JSON input is an object and the wizard is at the stream step
- **THEN** `to_entries[]` ranks higher than `.[]` in the suggestion list

#### Scenario: Selecting a simple stream advances to bind-keyword step

- **WHEN** the user accepts `.[]` at the stream step
- **THEN** the query becomes `foreach .[] ` and the suggestion dropdown shows bind-keyword options (`as`, `|`)

---

### Requirement: Parametric streams enter a sub-wizard before bind-keyword

When the user selects `range(0; 5)` or `recurse(.children[])`, the system SHALL enter a sub-wizard that steps through the stream's interior argument slots before advancing to the bind-keyword step.

#### Scenario: range sub-wizard — first slot (start)

- **WHEN** the user selects `range(0; 5)` at the stream step
- **THEN** the query becomes `foreach range(|0|; 5) ` with cursor at `0`
- **AND** the dropdown suggests small integer start values (`0`, `1`)

#### Scenario: range sub-wizard — second slot (end)

- **WHEN** the user accepts the start value and tabs
- **THEN** the cursor moves to the end value slot inside `range(0; |5|)`
- **AND** the dropdown suggests small integer end values (`5`, `10`, `100`)

#### Scenario: range sub-wizard completes to bind-keyword

- **WHEN** the user accepts the end value (or presses Enter to default)
- **THEN** the query becomes `foreach range(0; 5) ` and the dropdown shows bind-keyword options

#### Scenario: recurse sub-wizard — single slot (expr)

- **WHEN** the user selects `recurse(.children[])` at the stream step
- **THEN** the query becomes `foreach recurse(|.children[]|) ` with cursor at `.children[]`
- **AND** the dropdown suggests field path completions
- **WHEN** the user accepts or presses Enter
- **THEN** the query advances to the bind-keyword step

---

### Requirement: Bind-keyword step offers `as` (default) and `|`

At the bind-keyword step the system SHALL offer two options: `as` (canonical default) and `|` (pipe). Tab or Enter on `as` advances to the variable-name step.

#### Scenario: Accepting `as` advances to variable-name step

- **WHEN** the user accepts `as` at the bind-keyword step
- **THEN** the query becomes `foreach .[] as $` and the dropdown shows variable name candidates

#### Scenario: Accepting `|` inserts pipe and exits wizard

- **WHEN** the user accepts `|` at the bind-keyword step
- **THEN** the query becomes `foreach .[] | ` and wizard mode exits (normal completion resumes)

---

### Requirement: Variable-name step suggests context-derived names

The system SHALL suggest variable names at the `$<cursor>` position. Suggestions SHALL include default names (`$x`, `$item`, `$acc`) and any names already bound in the surrounding query (from outer `as` patterns or prior `foreach`/`reduce` expressions).

#### Scenario: Default variable names offered

- **WHEN** the wizard is at the variable-name step with no outer bound variables
- **THEN** the suggestions include `$x`, `$item`, `$acc` in that order

#### Scenario: Outer bound variables are offered first

- **WHEN** the query context already has a bound variable `$row` in scope
- **THEN** `$row` appears in the suggestion list alongside default names

#### Scenario: Accepting a variable name advances to init step

- **WHEN** the user accepts `$x`
- **THEN** the query becomes `foreach .[] as $x (` and the dropdown shows init expression suggestions

---

### Requirement: Init step suggests JSON-type-aware initial accumulator values

At the init step the system SHALL suggest initial accumulator expressions. The set of suggestions SHALL be informed by the current executor JSON input type.

#### Scenario: Numeric init suggestions for array/number input

- **WHEN** the JSON input is an array or number
- **THEN** the init suggestions include `0` (first/default), `null`, `[]`, `{}`

#### Scenario: Object init suggestion ranked first for object input

- **WHEN** the JSON input is an object
- **THEN** the init suggestions include `{}` (first/default), `null`, `0`, `[]`

#### Scenario: Accepting init advances to update-accumulator sub-step

- **WHEN** the user accepts `0` at the init step
- **THEN** the query becomes `foreach .[] as $x (0; ` and the dropdown shows accumulator prefix suggestions

---

### Requirement: Update step uses two sub-steps — accumulator then operation

The update expression slot is filled in two phases. Phase 1 (UpdateAccum) selects the accumulator prefix. Phase 2 (UpdateOp) selects the operation applied to it.

#### Scenario: UpdateAccum offers `.`, bound variable, and field paths

- **WHEN** the wizard is at the UpdateAccum sub-step with bound variable `$x`
- **THEN** the dropdown suggests: `.` (default, identity accumulator), `$x`, and top-level field paths from JSON context

#### Scenario: Accepting `.` in UpdateAccum advances to UpdateOp

- **WHEN** the user accepts `.` at UpdateAccum
- **THEN** the query becomes `foreach .[] as $x (0; .` and the dropdown shows operation suggestions relative to `.`

#### Scenario: UpdateOp suggests arithmetic and replacement operations

- **WHEN** the wizard is at the UpdateOp sub-step with bound variable `$x`
- **THEN** the dropdown suggests: `. + $x` (default), `. - $x`, `. * $x`, `. + 1`, `. - 1`, `$x` (replace with variable)

#### Scenario: Accepting an operation completes the update slot

- **WHEN** the user accepts `. + $x` at UpdateOp
- **THEN** for `reduce`, the query becomes `reduce .[] as $x (0; . + $x)` and wizard exits with cursor after `)`
- **THEN** for `foreach`, the query becomes `foreach .[] as $x (0; . + $x` and the extract step is shown

---

### Requirement: `foreach` extract step offers `)` (close) or `; <extract>` (continue)

After UpdateOp, `foreach` wizards SHALL show an extract-clause step. `reduce` wizards SHALL skip this step entirely.

#### Scenario: Selecting `)` closes the foreach clause

- **WHEN** the user accepts `)` at the extract step (or presses Enter for the default)
- **THEN** the query becomes `foreach .[] as $x (0; . + $x)` and wizard exits with cursor after `)`

#### Scenario: Selecting `; .` opens an extract slot

- **WHEN** the user accepts `; .` at the extract step
- **THEN** the query becomes `foreach .[] as $x (0; . + $x; .)` with cursor positioned at `.`
- **AND** wizard mode exits so the user can edit the extract expression freely

---

### Requirement: Enter at any wizard step fast-forwards to clause end with defaults

At any wizard step, pressing Enter SHALL apply the canonical default for the current slot and all remaining slots in sequence, then close the clause and position the cursor after `)`. The canonical full defaults are:
- Stream: `.[]`
- Bind-keyword: `as`
- Variable: `$x`
- Init: `0`
- Update: `. + $x`
- Extract (foreach): `)` (omit extract)

#### Scenario: Enter at stream step produces complete expression

- **WHEN** the user presses Enter at the stream step (first wizard step after the keyword)
- **THEN** the query becomes `foreach .[] as $x (0; . + $x)` (or `reduce ...`) with cursor after `)`

#### Scenario: Enter at init step completes from init onward

- **WHEN** the user has selected `.[]` as stream and `$x` as variable and presses Enter at the init step
- **THEN** the query becomes `foreach .[] as $x (0; . + $x)` with cursor after `)`

---

### Requirement: Esc walks back one wizard step

Pressing Esc while in wizard mode SHALL pop the current wizard step from the state stack, restore the query and suggestion list from the previous step, and re-show that step's dropdown. If Esc is pressed at the very first wizard step (stream selection), wizard mode exits entirely and all suggestions close.

#### Scenario: Esc at variable-name step returns to bind-keyword step

- **WHEN** the wizard is at the variable-name step (query: `foreach .[] as $`)
- **AND** the user presses Esc
- **THEN** the query reverts to `foreach .[] ` and the bind-keyword suggestion dropdown is re-shown

#### Scenario: Esc at init step returns to variable-name step

- **WHEN** the wizard is at the init step (query: `foreach .[] as $x (`)
- **AND** the user presses Esc
- **THEN** the query reverts to `foreach .[] as $` and the variable-name dropdown is re-shown

#### Scenario: Esc at first step (stream) exits wizard entirely

- **WHEN** the wizard is at the stream step
- **AND** the user presses Esc
- **THEN** the query reverts to `foreach ` (or `reduce `) and all suggestions close, wizard mode deactivated

---

### Requirement: Wizard produces syntactically valid jq at every step boundary

At every accepted slot boundary the accumulated query string SHALL be valid jq up to the current cursor position (i.e., a syntactically correct prefix). The wizard SHALL never produce dangling punctuation (e.g. unclosed `(` without the `;`) as a side effect of slot acceptance — incomplete structure is permitted only while the cursor is actively inside a slot.

#### Scenario: Partially built query is structurally coherent

- **WHEN** the user has accepted stream and bind-keyword and variable, placing cursor at `foreach .[] as $x (`
- **THEN** the partial string `foreach .[] as $x (` is a valid incomplete jq expression (the open `(` is intentional and expected)
- **AND** no spurious `;` or `)` appears before the user fills the init slot

---

### Requirement: Wizard state is discarded if the user edits the query manually

If the user types any character that is not a wizard-dispatched Tab/Enter/Esc while wizard mode is active, the system SHALL exit wizard mode and revert to normal suggestion behaviour.

#### Scenario: Typing a character exits wizard mode

- **WHEN** the wizard is at the init step and the user types `2` (a digit)
- **THEN** wizard mode is deactivated and normal suggestion logic applies from the current query state

---

### Requirement: Thorough unit test coverage for wizard transitions

The implementation SHALL include unit tests covering:
- Every slot transition for both `foreach` and `reduce`
- Esc stack pop at every step including the boundary (Esc from step 0 exits)
- Enter fast-forward from every step
- range sub-wizard (both slots) and recurse sub-wizard (single slot)
- Extract clause offer and both choices (`)`  and `; .`)
- Manual edit triggering wizard exit
- JSON-context influence on init and stream ranking (object vs array input)

#### Scenario: Transition tests pass for all steps

- **WHEN** the wizard unit tests are run
- **THEN** all step-transition, Esc, and Enter-fast-forward tests pass with no failures
