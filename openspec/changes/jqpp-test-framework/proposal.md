## Why

The project has no structured, script-executable test harness. Behaviour regressions in intellisense (suggestion box position, content, Tab/Enter/Esc acceptance flows) are caught only by manual TUI inspection, which is slow, inconsistent, and impossible to run in CI. A shared testing framework with a common directory layout, scenario format, and runner scripts will make regression detection automatic and repeatable across jqpp versions.

## What Changes

- Introduce a shared `skills/testing/` directory with a common directory layout for scenario data, screenshots, and baselines used by both `jqpp-tui-test` and `jqpp-scenario-gen`
- Define a canonical YAML scenario format that covers: jq query evaluation scenarios and TUI intellisense scenarios (suggestion content, position, key-acceptance flows)
- Add runner shell scripts that execute scenario files without an LLM and report pass/fail, suitable for CI
- Add a screenshot-extraction library (`scripts/tui/lib/screenshot.sh`) with reliable functions for: detecting the suggestion box, extracting its content lines, and reading current query bar text from agent-tui `--strip-ansi` output
- Add a scenario-generation workflow: given a jq function name, fetch documentation from jqlang.org/manual, generate JSON fixtures with known outputs via `jq`, write intellisense scenarios describing what the suggestion box should show at each keystroke, and validate them against the running TUI
- Add a query-exploration workflow: given a random or user-supplied JSON fixture, use `jq` to enumerate valid query paths and build scenario stubs for untested combinations
- Each scenario file records the jqpp version (semver + git SHA) it was validated against

## Capabilities

### New Capabilities

- `test-scenario-format`: Canonical YAML format for both jq evaluation and TUI intellisense scenarios, version-tagged, stored in `skills/testing/scenarios/`
- `tui-test-runner`: Shell scripts that launch jqpp via agent-tui, execute a scenario's keystroke sequence, capture suggestion box content and position, compare to expected values, and report pass/fail without LLM involvement
- `screenshot-lib`: Reusable shell functions for parsing agent-tui `--strip-ansi` output: detecting suggestion box presence, extracting suggestion labels, measuring box column position, reading query bar text
- `scenario-gen-from-jq-docs`: Workflow to generate test scenarios for a named jq function by fetching its documentation, inferring input/output pairs, and producing YAML scenario stubs for both evaluation and intellisense
- `query-exploration`: Workflow to enumerate valid jq query paths over a fixture, identify untested paths, and generate scenario stubs for them

### Modified Capabilities

- `param-field-completions`: Intellisense scenarios added as test data; no spec-level requirement change
- `suggestion-activation`: TUI positioning test added as test data; no spec-level requirement change

## Impact

- New directory `skills/testing/` (shared between skill files via symlinks already in place)
- New directory `scripts/tui/` for runner and library scripts
- New directory `skills/testing/scenarios/` for scenario YAML files
- New directory `skills/testing/baselines/` for text-snapshot baselines
- agent-tui binary must be available on PATH; `jq` must be available on PATH
- No changes to jqpp Rust source code in this change
- Skills `jqpp-tui-test` and `jqpp-scenario-gen` updated to reference shared paths
