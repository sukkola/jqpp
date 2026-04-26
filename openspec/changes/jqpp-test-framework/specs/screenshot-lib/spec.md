## ADDED Requirements

### Requirement: Suggestion box detection function
`scripts/tui/lib/screenshot.sh` SHALL expose a function `tui_suggestion_box_visible SCREEN` that returns exit code 0 if a suggestion box is present in the stripped ANSI screenshot text and 1 if not. The suggestion box is identified by the presence of a `│` character on a non-border line within the content area.

#### Scenario: Suggestion box present
- **WHEN** the screenshot contains lines with `│content│` pattern inside the terminal area
- **THEN** `tui_suggestion_box_visible` returns 0

#### Scenario: No suggestion box
- **WHEN** the screenshot contains only the Query and Output pane borders with no inner `│` lines
- **THEN** `tui_suggestion_box_visible` returns 1

### Requirement: Suggestion label extraction function
`scripts/tui/lib/screenshot.sh` SHALL expose a function `tui_extract_suggestions SCREEN` that prints one suggestion label per line, trimmed of leading and trailing whitespace, excluding blank lines and lines that are entirely box-drawing characters.

#### Scenario: Extract three suggestions
- **WHEN** the screenshot shows a dropdown with three rows: `│ orders │`, `│ metadata │`, `│ user │`
- **THEN** `tui_extract_suggestions` prints: `orders`, `metadata`, `user` (one per line)

#### Scenario: Exclude separator lines
- **WHEN** a row contains only `─` characters inside `│` borders
- **THEN** that row is not included in the output

### Requirement: Query bar text extraction function
`scripts/tui/lib/screenshot.sh` SHALL expose a function `tui_read_query SCREEN` that returns the current text content of the Query input bar, trimmed of border characters and surrounding whitespace.

#### Scenario: Query bar with text
- **WHEN** the screenshot shows `│ .orders[] │` in the Query bar row
- **THEN** `tui_read_query` returns `.orders[]`

#### Scenario: Empty query bar
- **WHEN** the query bar row contains only whitespace between `│` characters
- **THEN** `tui_read_query` returns an empty string

### Requirement: All functions accept SCREEN as argument (not stdin)
Functions SHALL accept the screen content as a bash variable argument so they can be called multiple times on the same captured output without re-running agent-tui. The caller captures once with `SCREEN=$(agent-tui screenshot --strip-ansi)` and passes `"$SCREEN"` to each function.

#### Scenario: Multiple assertions from single capture
- **WHEN** a scenario checks both `tui_suggestion_box_visible "$SCREEN"` and `tui_extract_suggestions "$SCREEN"` 
- **THEN** both calls work correctly from the same captured string without additional agent-tui invocations
