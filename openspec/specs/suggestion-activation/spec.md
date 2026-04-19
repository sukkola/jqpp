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
