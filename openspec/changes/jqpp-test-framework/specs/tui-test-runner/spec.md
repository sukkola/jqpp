## ADDED Requirements

### Requirement: Standalone runner script
`scripts/tui/run-scenarios.sh` SHALL be executable without an LLM. It SHALL accept a path argument (file or directory), convert scenario YAML files to JSON via `scripts/tui/lib/yaml2json.sh`, parse them with `jaq`, launch jqpp via `agent-tui`, execute each TUI scenario's action sequence, evaluate assertions, and print a summary of `PASS`, `FAIL`, and `EXPECTED FAIL` counts. Exit code SHALL be 0 if all non-regression scenarios pass, non-zero otherwise. The runner SHALL NOT depend on `yq`.

#### Scenario: All scenarios pass
- **WHEN** all TUI and evaluation scenarios in the target file(s) pass their assertions
- **THEN** the runner prints `Results: N passed, 0 failed` and exits 0

#### Scenario: One scenario fails
- **WHEN** one TUI scenario's `suggestions_contain` assertion fails
- **THEN** the runner prints `FAIL: <id>` with the actual suggestion list, continues remaining scenarios, and exits 1 after the summary

#### Scenario: Agent-tui unavailable
- **WHEN** `agent-tui` is not on PATH (not installed via mise)
- **THEN** the runner prints `ERROR: agent-tui not found. Run: mise install` and exits 1 immediately

#### Scenario: jaq unavailable
- **WHEN** `jaq` is not on PATH
- **THEN** the runner prints `ERROR: jaq not found. Run: mise install` and exits 1 immediately

### Requirement: Runner captures jqpp version before executing
Before running any TUI scenarios the runner SHALL capture `jqpp_version` and `jqpp_git_sha` from the current build and print them in the run header. It SHALL warn (but not fail) if they differ from the `meta.jqpp_version` in any scenario file.

#### Scenario: Version mismatch warning
- **WHEN** the scenario file was created against `0.3.0` but the running binary reports `0.4.0`
- **THEN** the runner prints `WARN: scenario created against 0.3.0, running against 0.4.0` before that file's results

### Requirement: TUI lifecycle management per scenario
For each TUI scenario the runner SHALL: launch a fresh jqpp session with `agent-tui run`, wait for the Query bar to appear, execute the action sequence, capture the final screenshot, evaluate assertions, and kill the session — even if assertions fail.

#### Scenario: Session cleanup on failure
- **WHEN** an assertion fails mid-scenario
- **THEN** the runner still calls `agent-tui kill` for that session before moving to the next scenario

#### Scenario: Session ready check
- **WHEN** jqpp is launched
- **THEN** the runner waits with `agent-tui wait "Query" --assert -t 10000` before sending any actions; if the wait times out the scenario is marked `ERROR: timeout waiting for TUI` and the session is killed

### Requirement: Evaluation scenarios run without agent-tui
Evaluation scenarios (jq filter + input + expected output) SHALL be executed using the `jq` binary only, with no agent-tui session. The runner SHALL detect the scenario type from the presence of `filter` vs `actions` keys.

#### Scenario: Evaluation scenario runs jq directly
- **WHEN** a scenario has a `filter` key
- **THEN** the runner runs `echo "$input" | jq "$filter"` and compares to `expected_output`
