## ADDED Requirements

### Requirement: --query sets the initial query bar content
The system SHALL accept a `--query <expr>` CLI flag whose value is set verbatim as the contents of the query bar before the first TUI frame is drawn.

#### Scenario: Query bar pre-filled on startup
- **WHEN** jqpp is launched with `--query '.foo'`
- **THEN** the query bar shows `.foo` when the TUI first appears

#### Scenario: Empty --query is valid and leaves bar empty
- **WHEN** jqpp is launched with `--query ''`
- **THEN** the query bar is empty, equivalent to launching without the flag

#### Scenario: --query with no --cursor places cursor at end
- **WHEN** jqpp is launched with `--query '.items[]'` and no `--cursor` flag
- **THEN** the cursor is positioned after the last character of the query

#### Scenario: --query works without any input data
- **WHEN** jqpp is launched with `--query '.foo'` and no file or pipe input
- **THEN** the query bar shows `.foo` and the tool starts normally with an empty input pane

### Requirement: Suggestions activate immediately for non-trivial pre-filled query
The system SHALL trigger suggestion computation before the first TUI frame when `--query` produces a query that would normally show suggestions.

#### Scenario: Suggestions visible on first frame
- **WHEN** jqpp is launched with a file and `--query '.items[] | .na'`
- **THEN** field-name suggestions matching `.na` are visible without any user keypress

#### Scenario: No spurious suggestions for empty query
- **WHEN** jqpp is launched with `--query ''` or without `--query`
- **THEN** no suggestion dropdown is shown on the first frame

### Requirement: --cursor sets the initial cursor column with positive or negative index
The system SHALL accept a `--cursor <col>` CLI flag (signed integer) that positions the cursor within the pre-filled query on startup. Positive values are 0-based character offsets from the start; negative values count from the end of the query string (`-1` = one before the end, i.e. after the last character).

#### Scenario: Positive cursor placed at specified column
- **WHEN** jqpp is launched with `--query 'sort_by(.price)' --cursor 9`
- **THEN** the cursor is inside the parentheses after `sort_by(`, at column 9

#### Scenario: Negative cursor counts from the end
- **WHEN** jqpp is launched with `--query 'sort_by(.price)' --cursor -7`
- **THEN** the cursor is placed 7 characters before the end of the query, inside `.price`

#### Scenario: --cursor -1 places cursor after the last character
- **WHEN** jqpp is launched with `--query '.foo'` and `--cursor -1`
- **THEN** the cursor is at column 4, one position before the end (same as the last character position), which equals the character count of the query

#### Scenario: Negative cursor more negative than query length clamps to start
- **WHEN** jqpp is launched with `--query '.a'` and `--cursor -999`
- **THEN** the cursor is placed at column 0 (start of the query), no error or warning

#### Scenario: --cursor without --query is silently ignored
- **WHEN** jqpp is launched with `--cursor 3` but no `--query` flag
- **THEN** the cursor starts at column 0 (start of the empty query bar), no error

#### Scenario: Positive --cursor value larger than query length is clamped
- **WHEN** jqpp is launched with `--query '.a'` and `--cursor 999`
- **THEN** the cursor is placed at the end of the query (column 2), no error or warning

#### Scenario: Suggestions fire at --cursor position using negative index
- **WHEN** jqpp is launched with a file, `--query 'sort_by(.pr)'`, and `--cursor -1`
- **THEN** param-field completions for `sort_by` are visible on the first frame, with the cursor inside the parentheses at the end of the query

### Requirement: --query is compatible with --print-* output flags
The system SHALL allow `--query` to be combined with any `--print-*` flag.

#### Scenario: --query with --print-output exits after computing
- **WHEN** jqpp is launched with `--query '.name' --print-output` in headless mode
- **THEN** the query `.name` is evaluated against the input and the result is written to stdout

#### Scenario: --query with --print-query reflects the initial query
- **WHEN** jqpp is launched with `--query '.items[]' --print-query` in headless mode
- **THEN** `.items[]` is written to stdout
