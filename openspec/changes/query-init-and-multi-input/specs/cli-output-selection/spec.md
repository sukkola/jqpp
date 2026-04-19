## MODIFIED Requirements

### Requirement: --print-result outputs query result on exit
The system SHALL write the current query result to stdout after the TUI exits when `--print-result` is passed on the command line. The output SHALL use the same newline-delimited JSON format used by Ctrl+S save-to-file. This flag is compatible with multiple file arguments and with `--query`; when multiple inputs are merged, the result reflects the query applied to the merged array.

#### Scenario: Result printed after normal quit
- **WHEN** jqpp is launched with `--print-result` and the user quits normally (Ctrl+C or q)
- **THEN** the current query result is written to stdout, followed by a newline, after the terminal is restored

#### Scenario: Empty output when query has errors
- **WHEN** jqpp is launched with `--print-result` and the active query has a parse or evaluation error at exit
- **THEN** nothing is written to stdout (empty output, zero bytes)

#### Scenario: Raw output preserved
- **WHEN** the query uses a format operator (e.g. `@csv`) and `--print-result` is active
- **THEN** the raw string output is printed, not JSON-encoded

#### Scenario: Result printed for merged multi-file input
- **WHEN** jqpp is launched with two files and `--print-result` and the user quits
- **THEN** the result of the query applied to the merged array is written to stdout

## ADDED Requirements

### Requirement: Positional file argument accepts zero or more paths
The `file` positional argument SHALL accept zero or more file paths. Zero paths keeps existing no-input behaviour; one path keeps existing single-file behaviour; two or more paths activates multi-file mode.

#### Scenario: No positional arguments accepted
- **WHEN** jqpp is launched with no positional arguments and no stdin pipe
- **THEN** jqpp starts with an empty input pane, same as before

#### Scenario: One positional argument accepted
- **WHEN** jqpp is launched with exactly one positional file path
- **THEN** jqpp loads that file as the dataset, same as before

#### Scenario: Two or more positional arguments accepted
- **WHEN** jqpp is launched with two or more positional file paths
- **THEN** jqpp loads all files and merges them into a JSON array dataset
