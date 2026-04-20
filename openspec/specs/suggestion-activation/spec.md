# suggestion-activation Specification

## Purpose
Define when the suggestion dropdown activates and deactivates as the user edits the query. The principle is that the dropdown should only appear when there is a clear, contextually useful trigger — not as a side-effect of navigating the UI (e.g. pressing Down on an empty box) or simply starting to type free-form text.

## Requirements

### Requirement: Trigger characters always open suggestions
The system SHALL activate suggestion mode when the user types a jq structural character (`.`, `|`, `{`, `[`, `,`, `@`) or presses Backspace/Delete. These keys have unambiguous intent to navigate or modify a query path.

#### Scenario: Dot opens field completions
- **WHEN** the user types `.` in the query box
- **THEN** the suggestion dropdown activates

#### Scenario: Pipe opens function completions
- **WHEN** the user types `|` in the query box
- **THEN** the suggestion dropdown activates

### Requirement: Alphanumeric typing after a trigger continues suggestions
The system SHALL keep suggestion mode active when the user types alphanumeric characters (or `_`, `-`, space) AND the current query has a trigger character preceding the word being typed. This allows filtering of field names and function names after `.` or `|`.

#### Scenario: Filtering a field name
- **WHEN** the query prefix is `.foo` (typed `f`, `o`, `o` after a `.`)
- **THEN** suggestion mode remains active throughout

#### Scenario: Filtering a function name after pipe
- **WHEN** the query prefix is `.x|sel` (typing `sel` after `|`)
- **THEN** suggestion mode remains active

### Requirement: Typing alphanumeric characters always activates suggestions
The system SHALL activate suggestion mode when the user types any alphanumeric character, underscore, dash, or space — regardless of what preceded it. This means the dropdown appears from the first keystroke so users do not need to press Down or type a structural character first. For example, typing `c` in an empty box immediately offers field completions such as `.created`.

#### Scenario: Typing from an empty query box
- **WHEN** the query box is empty and the user types `c`
- **THEN** suggestion mode is activated and completions that match `c` (e.g. `.created`) appear in the dropdown

#### Scenario: No need to press Down before typing
- **WHEN** the user starts typing a bare identifier without pressing Down first
- **THEN** suggestion mode is activated, identical to typing after pressing Down

### Requirement: Bare token matches top-level field names by prefix
The system SHALL include top-level JSON field names in the suggestion dropdown when the user types a bare alphanumeric token (no leading `.`) that is a prefix of one or more field names. The insert text for such completions SHALL include the leading `.` so accepting the suggestion produces a valid jq path.

#### Scenario: Bare letter surfaces field completions
- **WHEN** the input has a top-level field `created` and the user types `c`
- **THEN** `created` appears in the suggestion dropdown with insert text `.created`

#### Scenario: Longer bare prefix narrows field completions
- **WHEN** the user types `cre`
- **THEN** only fields beginning with `cre` (e.g. `created`) appear; fields like `count` do not

#### Scenario: Field completions rank before unrelated builtins
- **WHEN** the user types `c` and both `.created` and builtin `contains()` match
- **THEN** `.created` (exact field prefix match) appears before fuzzy-matched builtins in the list

### Requirement: Explicit Down navigation opens the dropdown
The system SHALL also open the suggestion dropdown when the user presses Down (or Up) in the query box, for users who prefer explicit invocation.

### Requirement: Function acceptance starts from empty argument position
When a function completion is accepted (Tab/Enter), the inserted text SHALL place the cursor at the next user-input position and SHALL NOT prefill semantic placeholder arguments (for example object keys, indices, or sample literals). Function calls with arguments therefore insert as empty parentheses (`fn()`) and rely on follow-up contextual suggestions to guide argument entry.

#### Scenario: has inserts empty parens
- **WHEN** the user accepts the `has` completion
- **THEN** the query inserts `has()` and the cursor lands between `(` and `)`

#### Scenario: contains inserts empty parens
- **WHEN** the user accepts the `contains` completion
- **THEN** the query inserts `contains()` and the cursor lands between `(` and `)`

#### Scenario: contextual suggestions appear immediately after function acceptance
- **WHEN** the user is positioned inside `has(` or `contains(` after accepting a completion
- **THEN** the dropdown offers context-aware argument suggestions without requiring manual deletion of prefilled placeholders

### Requirement: Builder-style `contains` suggestions use key/value and Tab/Enter flow
For `contains` on array/object inputs, the system SHALL guide users through incremental builder-style argument construction.

#### Scenario: `contains` object suggestions are key-first then value
- **WHEN** the user types `contains({` in object context
- **THEN** suggestions list candidate keys first (without prefilled values)
- **AND WHEN** a key is accepted
- **THEN** suggestions switch to candidate values for that key

#### Scenario: `contains` array suggestions append one value at a time
- **WHEN** the user is inside `contains([` in array context
- **THEN** suggestions list scalar array values to append as comparison elements

#### Scenario: Tab continues builder flow, Enter finalizes current selection set
- **WHEN** a `contains` value suggestion is accepted with Tab
- **THEN** the selected value is inserted and a trailing comma is added so suggestions continue for the next element/field
- **WHEN** a `contains` value suggestion is accepted with Enter
- **THEN** the enclosing array/object argument is closed and cursor moves after `)`

#### Scenario: Esc exits `contains` builder suggestions without implicit approval
- **WHEN** the user presses Esc during `contains` key/value builder suggestions
- **THEN** suggestions close
- **AND** no additional key/value is auto-inserted

#### Scenario: Builder output remains structurally valid on approval
- **WHEN** Enter finalizes `contains` builder content
- **THEN** object mode ensures surrounding `{}` and array mode ensures surrounding `[]`
- **AND** the resulting `contains(...)` argument is syntactically complete before cursor moves past `)`
