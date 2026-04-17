## Why

jqpp is interactive, but the result of a session — the refined query, the filtered output, or the raw input — cannot currently be captured for downstream use. Adding mutually exclusive `--print-*` flags lets jqpp participate in shell pipelines: the user explores interactively, presses Enter to confirm, and the chosen content flows to stdout on exit.

## What Changes

- Add `--print-result` flag: on exit, write the current query result (right pane content) to stdout
- Add `--print-query` flag: on exit, write the current query string to stdout
- Add `--print-input` flag: on exit, write the raw JSON input (left pane content) to stdout
- Exactly one flag may be active at a time; combining any two is a usage error that prints a message and exits non-zero without starting the TUI
- When a `--print-*` flag is active, the selected content is written to stdout after the TUI restores the terminal, before the process exits
- Normal TUI rendering and interaction are unchanged; the flag only affects what happens at exit

## Capabilities

### New Capabilities

- `cli-output-selection`: CLI flags for selecting which pane's content is emitted to stdout on exit, with mutual-exclusion enforcement and documentation

### Modified Capabilities

(none)

## Impact

- `src/main.rs`: `Args` struct gains three `bool` fields; mutual-exclusion check in `actual_main`; stdout write after TUI teardown
- `src/app.rs`: `App` stores the selected output mode so the exit path can retrieve the right content without re-parsing args
- `README.md`: Usage section updated with `--print-result`, `--print-query`, `--print-input` examples; flags documented in a new "Pipeline usage" subsection
- No new dependencies
