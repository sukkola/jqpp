## ADDED Requirements

### Requirement: Scenario files record jqpp version
Every scenario YAML file SHALL include a `meta` block with `jqpp_version` (semver string), `jqpp_git_sha` (short SHA), `created` (ISO date), `topic`, and `description` fields. Files missing any of these fields SHALL be rejected by the runner.

#### Scenario: Valid meta block
- **WHEN** a scenario file has `meta.jqpp_version`, `meta.jqpp_git_sha`, `meta.created`, `meta.topic`, and `meta.description`
- **THEN** the runner accepts the file and proceeds to execute scenarios

#### Scenario: Missing version field
- **WHEN** a scenario file has no `meta.jqpp_version` field
- **THEN** the runner exits with a non-zero status and prints `ERROR: missing meta.jqpp_version in <file>`

### Requirement: Evaluation scenario format
An evaluation scenario SHALL be a YAML object with: `id` (unique string), `description` (string), `input` (JSON string), `filter` (filter string), `expected_output` (string, normalized trailing whitespace), and optional `tags` (list of strings), `status` (`active` | `regression` | `skip`), and `jq_diverges` (boolean, default false).

The `jq_diverges` flag marks scenarios where `jaq` and `jq` produce different output. The runner SHALL execute these with `jaq` as primary and report the `jq` output separately for reference.

#### Scenario: Passing evaluation scenario
- **WHEN** `echo "$input" | jaq "$filter"` produces output matching `expected_output` after whitespace normalization
- **THEN** the runner reports `PASS: <id>`

#### Scenario: jaq/jq divergence flagged
- **WHEN** `jaq` and `jq` produce different output for the same filter and `jq_diverges: true` is set
- **THEN** the runner reports `PASS (jaq): <id>` and `INFO: jq produces: <jq_output>` without failing

#### Scenario: Unexpected jaq/jq divergence
- **WHEN** `jaq` and `jq` produce different output but `jq_diverges` is not set
- **THEN** the runner reports `WARN: jaq/jq diverge on <id>` and suggests setting `jq_diverges: true`

#### Scenario: Regression-tagged scenario
- **WHEN** `status: regression` is set on a scenario
- **THEN** the runner treats a failure as `EXPECTED FAIL: <id>` and does not count it toward the failure total; a pass is reported as `UNEXPECTED PASS: <id>` and counted as a failure

### Requirement: TUI intellisense scenario format
A TUI intellisense scenario SHALL be a YAML object with: `id`, `description`, `input_file` (path relative to repo root), `actions` (ordered list of action objects), and `assertions` (list of assertion objects). Actions are one of: `type` (string), `press` (key name), `wait_stable` (boolean). Assertions are one of: `suggestions_visible` (boolean), `suggestions_contain` (list of strings), `suggestions_not_contain` (list of strings), `query_contains` (string).

#### Scenario: TUI scenario with keystroke sequence
- **WHEN** a TUI scenario has `actions: [{type: "."}, {press: "Tab"}]`
- **THEN** the runner types `.` into the query bar and presses Tab in that order

#### Scenario: Suggestion content assertion
- **WHEN** an assertion `suggestions_contain: ["orders"]` is present
- **THEN** the runner extracts suggestion box content from the screenshot and checks that `orders` appears in the list; failure is reported if not found

### Requirement: Scenario files processed via YAML-to-JSON conversion
Scenario YAML files SHALL be converted to JSON using a lightweight helper (`scripts/tui/lib/yaml2json.sh`, implemented as a Python yaml→json one-liner) before being parsed by `jaq`. Scripts SHALL NOT depend on `yq`.

#### Scenario: YAML file converted before parsing
- **WHEN** a scenario file at `skills/testing/scenarios/select.yaml` is passed to the runner
- **THEN** the runner converts it to JSON with `yaml2json`, then uses `jaq` to extract scenario fields

### Requirement: Scenario-to-screenshot binding
Every TUI intellisense scenario SHALL declare which screenshot lib functions apply to it via an `asserts_with` list in the scenario YAML. Valid values are the function names exported by `scripts/tui/lib/screenshot.sh`: `tui_suggestion_box_visible`, `tui_extract_suggestions`, `tui_read_query`. The runner SHALL call only the functions listed in `asserts_with` when evaluating that scenario, and SHALL skip assertion types not listed. Baseline files SHALL be named `<scenario-id>-<function-name>.txt` so each captured output is traceable to both the scenario and the lib function that produced it.

#### Scenario: Scenario lists applicable lib functions
- **WHEN** a scenario has `asserts_with: [tui_extract_suggestions, tui_read_query]`
- **THEN** the runner calls those two functions on the captured screenshot and evaluates the corresponding assertions; `tui_suggestion_box_visible` is not called

#### Scenario: Baseline file named by scenario and function
- **WHEN** a baseline is saved for scenario `dot-orders-tab` using `tui_extract_suggestions`
- **THEN** the baseline file is written to `skills/testing/baselines/dot-orders-tab-tui_extract_suggestions.txt`

#### Scenario: Missing asserts_with field
- **WHEN** a TUI scenario has no `asserts_with` field
- **THEN** the runner defaults to calling all three lib functions and warns `WARN: asserts_with not set for <id>; running all lib functions`

### Requirement: Screenshot capture recorded in scenario output
When the runner executes a TUI scenario it SHALL write the raw `--strip-ansi` screenshot to `skills/testing/baselines/<scenario-id>-raw.txt` alongside any function-specific extracts. This links each stored baseline back to the exact scenario run that produced it.

#### Scenario: Raw screenshot stored after run
- **WHEN** scenario `select-dot-suggests-fields` is executed
- **THEN** `skills/testing/baselines/select-dot-suggests-fields-raw.txt` is written with the full stripped screenshot

#### Scenario: Baseline updated on explicit request
- **WHEN** `run-scenarios.sh --update-baselines` flag is passed
- **THEN** existing baseline files are overwritten with current output instead of compared

### Requirement: Shared scenario directory layout
All scenario YAML files SHALL reside under `skills/testing/scenarios/`. Intellisense-specific scenarios SHALL be in `skills/testing/scenarios/intellisense/`. Baseline text snapshots SHALL be in `skills/testing/baselines/` named `<scenario-id>-<function-name>.txt` (function-specific) and `<scenario-id>-raw.txt` (full screenshot).

#### Scenario: Runner discovers scenario files
- **WHEN** `run-scenarios.sh` is called with a directory path
- **THEN** it finds all `*.yaml` files recursively and runs them in sorted order

#### Scenario: Runner called with a single file
- **WHEN** `run-scenarios.sh skills/testing/scenarios/select.yaml` is called
- **THEN** only scenarios in that file are executed
