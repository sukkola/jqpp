## ADDED Requirements

### Requirement: Generate evaluation scenarios for a named jq function
`scripts/tui/gen-scenarios.sh <function-name>` SHALL produce a YAML scenario file at `skills/testing/scenarios/<function-name>.yaml` containing at least one evaluation scenario per distinct input type (number, string, array of primitives, array of objects, object). Each scenario SHALL be validated by running it through `jq` before being written; invalid scenarios SHALL be skipped with a warning.

#### Scenario: Valid scenario written
- **WHEN** `jq "$filter" <<< "$input"` exits 0 and produces output matching `expected_output`
- **THEN** the scenario is appended to the output YAML file

#### Scenario: Invalid jq filter skipped
- **WHEN** `jq "$filter" <<< "$input"` exits non-zero
- **THEN** the scenario is not written and the script prints `SKIP: invalid filter: $filter`

### Requirement: Version metadata captured at generation time
Before writing any scenario file `gen-scenarios.sh` SHALL capture `jqpp_version` and `jqpp_git_sha` from the current debug build and embed them in the `meta` block.

#### Scenario: Version captured from binary
- **WHEN** `target/debug/jqpp --version` exits 0
- **THEN** its output is stored as `meta.jqpp_version` in the generated file

#### Scenario: Version fallback to cargo metadata
- **WHEN** `target/debug/jqpp` does not exist
- **THEN** `cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version'` is used as the fallback version string

### Requirement: Generate intellisense scenario stubs alongside evaluation scenarios
For each evaluation scenario generated, `gen-scenarios.sh` SHALL also produce a corresponding intellisense scenario stub in `skills/testing/scenarios/intellisense/<function-name>-intellisense.yaml`. The stub SHALL pre-populate `actions` with `type` steps that type the filter prefix character by character up to the opening `(` (if present), and leave `assertions` with `suggestions_visible: true` and empty `suggestions_contain` and `suggestions_not_contain` lists as placeholders for human or LLM review.

#### Scenario: Intellisense stub for select()
- **WHEN** generating scenarios for function `select`
- **THEN** an intellisense scenario stub is created with `actions: [{type: "s"}, {type: "e"}, {type: "l"}, {type: "e"}, {type: "c"}, {type: "t"}, {type: "("}]` and `assertions: [{suggestions_visible: true}]`

### Requirement: Dependencies managed via mise
`gen-scenarios.sh` SHALL check that `jaq` is available (installed via `mise`) before proceeding and exit with `ERROR: run 'mise install' to install dependencies` if not. `jq` is used as a secondary validator and the script SHALL warn but not exit if `jq` is absent.

#### Scenario: jaq available
- **WHEN** `jaq` is on PATH
- **THEN** the script proceeds normally

#### Scenario: jaq missing
- **WHEN** `jaq` is not on PATH
- **THEN** the script prints `ERROR: jaq not found. Run: mise install` and exits 1

#### Scenario: jq absent (secondary validator)
- **WHEN** `jq` is not on PATH
- **THEN** the script prints `WARN: jq not found; divergence checking disabled` and continues using jaq only
