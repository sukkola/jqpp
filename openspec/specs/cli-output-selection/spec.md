# cli-output-selection Specification

## Purpose
TBD - created by archiving change cli-output-flags. Update Purpose after archive.
## Requirements
### Requirement: --print-result outputs query result on exit
The system SHALL write the current query result to stdout after the TUI exits when `--print-result` is passed on the command line. The output SHALL use the same newline-delimited JSON format used by Ctrl+S save-to-file.

#### Scenario: Result printed after normal quit
- **WHEN** jqpp is launched with `--print-result` and the user quits normally (Ctrl+C or q)
- **THEN** the current query result is written to stdout, followed by a newline, after the terminal is restored

#### Scenario: Empty output when query has errors
- **WHEN** jqpp is launched with `--print-result` and the active query has a parse or evaluation error at exit
- **THEN** nothing is written to stdout (empty output, zero bytes)

#### Scenario: Raw output preserved
- **WHEN** the query uses a format operator (e.g. `@csv`) and `--print-result` is active
- **THEN** the raw string output is printed, not JSON-encoded

### Requirement: --print-query outputs query string on exit
The system SHALL write the current query bar contents to stdout after the TUI exits when `--print-query` is passed on the command line.

#### Scenario: Query string printed after quit
- **WHEN** jqpp is launched with `--print-query` and the user quits
- **THEN** the exact string from the query bar is written to stdout followed by a newline

#### Scenario: Empty query bar produces empty output
- **WHEN** the query bar is empty at exit and `--print-query` is active
- **THEN** a single newline is written to stdout

### Requirement: --print-input outputs raw JSON input on exit
The system SHALL write the full raw JSON input (as loaded from file or stdin) to stdout after the TUI exits when `--print-input` is passed on the command line.

#### Scenario: Input printed after quit
- **WHEN** jqpp is launched with `--print-input` and the user quits
- **THEN** the raw JSON string passed as input is written to stdout followed by a newline

### Requirement: Flags are mutually exclusive
The system SHALL reject any invocation that specifies more than one of `--print-result`, `--print-query`, or `--print-input`. The process SHALL print an error message to stderr and exit with a non-zero status code without starting the TUI.

#### Scenario: Two flags given at the same time
- **WHEN** the user runs `jqpp data.json --print-result --print-query`
- **THEN** jqpp prints an error message to stderr such as "error: only one --print-* flag may be used at a time" and exits with status 1 without opening the TUI

#### Scenario: Single flag is accepted
- **WHEN** exactly one of the three flags is given
- **THEN** jqpp starts normally and prints the selected content on exit

### Requirement: No --print-* flag leaves behaviour unchanged
The system SHALL behave identically to the current behaviour when none of the three flags are provided.

#### Scenario: Normal launch without flags
- **WHEN** jqpp is launched without any `--print-*` flag
- **THEN** nothing extra is written to stdout on exit

### Requirement: README documents pipeline usage
The README SHALL include a "Pipeline usage" section or equivalent with examples of each `--print-*` flag used in a shell pipeline.

#### Scenario: Examples present in README
- **WHEN** a user reads the README
- **THEN** they find at least one shell pipeline example for `--print-result`, `--print-query`, and `--print-input`

