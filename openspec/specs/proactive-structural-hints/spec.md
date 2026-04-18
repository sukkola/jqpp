# proactive-structural-hints Specification

## Purpose
TBD - created by archiving change contextual-ghost-suggestions. Update Purpose after archive.
## Requirements
### Requirement: Array context triggers [] ghost suggestion
The system SHALL automatically display `[]` as inline gray ghost text after the cursor when the current query prefix resolves to an array value in the live JSON input, without the user having to type any trigger character.

#### Scenario: Query resolves to array — ghost shown
- **WHEN** the user has typed `.items` and `.items` in the JSON input is an array
- **THEN** `[]` appears as gray ghost text immediately after the cursor

#### Scenario: Ghost not shown when query already ends with [
- **WHEN** the query already ends with `[`
- **THEN** no structural ghost text is shown (normal completion dropdown takes over)

#### Scenario: Ghost not shown for scalar result
- **WHEN** the query resolves to a string, number, boolean, or null value
- **THEN** no structural ghost text is shown

### Requirement: Object-array context triggers . ghost suggestion
The system SHALL automatically display `.` as inline gray ghost text when the query ends with `[]` and the first element of the iterated array is an object.

#### Scenario: Array of objects — dot ghost shown
- **WHEN** the user has typed `.items[]` and the elements of `items` are objects
- **THEN** `.` appears as gray ghost text after the cursor

#### Scenario: Array of scalars — no dot ghost
- **WHEN** the user has typed `.tags[]` and the elements of `tags` are strings
- **THEN** no ghost text is shown after `[]`

### Requirement: Tab accepts structural ghost suggestion
The system SHALL insert the ghost-suggested structural text into the query bar when Tab is pressed while a structural ghost suggestion is displayed.

#### Scenario: Tab inserts [] from ghost
- **WHEN** `[]` is shown as ghost text and the user presses Tab
- **THEN** `[]` is appended to the query, cursor moves to end, ghost clears

#### Scenario: Tab inserts . from ghost
- **WHEN** `.` is shown as ghost text and the user presses Tab
- **THEN** `.` is appended to the query and field completions open (dropdown shows object fields)

### Requirement: Up/Down opens full context dropdown from ghost suggestion
The system SHALL transition from ghost text display to the full suggestion dropdown when the user presses Up or Down while a structural ghost suggestion is active.

#### Scenario: Down arrow on [] ghost opens bracket dropdown
- **WHEN** `[]` is shown as ghost text and the user presses Down
- **THEN** the suggestion dropdown opens showing all array-context completions (e.g. `[]`, `[0]`, `[1]`, …) as if the user had typed `[`

#### Scenario: Up arrow behaves symmetrically
- **WHEN** `[]` is shown as ghost text and the user presses Up
- **THEN** the same dropdown opens with the last item highlighted

### Requirement: Esc dismisses structural ghost suggestion
The system SHALL dismiss the structural ghost suggestion when Esc is pressed, leaving the query unchanged.

#### Scenario: Esc clears ghost without modifying query
- **WHEN** `[]` is shown as ghost text and the user presses Esc
- **THEN** the ghost text disappears and the query bar shows only the text the user typed, with no modification

#### Scenario: Ghost does not reappear until query changes
- **WHEN** the user has dismissed a structural ghost with Esc
- **THEN** the ghost for the same query position does not reappear until the query string changes

### Requirement: Ghost reappears after backspace to triggering context
The system SHALL re-evaluate and re-show a structural ghost suggestion when the user backspaces back to a position where the ghost condition is met.

#### Scenario: Backspace to array field re-shows [] ghost
- **WHEN** the user had `.items` with a `[]` ghost, typed further to `.items[0]`, then backspaces back to `.items`
- **THEN** the `[]` ghost reappears

### Requirement: Structural ghost suppressed during user-initiated completion
The system SHALL not show structural ghost text when the user has actively triggered the completion dropdown by typing.

#### Scenario: Ghost suppressed while dropdown open
- **WHEN** the suggestion dropdown is open from user typing (e.g. after typing `.it`)
- **THEN** no structural ghost text is shown, even if the context would trigger one

### Requirement: Structural ghost suppressed for empty query
The system SHALL not show any structural ghost text when the query bar is empty.

#### Scenario: Empty query shows no ghost
- **WHEN** the query bar contains no text
- **THEN** no ghost text is displayed

