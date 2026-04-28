## ADDED Requirements

### Requirement: Enumerate testable query paths from a JSON fixture
`scripts/tui/explore-paths.sh <fixture.json>` SHALL use `jq` to enumerate all leaf and intermediate paths in the fixture, produce one candidate filter per path (e.g. `.orders[0].customer.name`), and write unevaluated scenario stubs to `skills/testing/scenarios/explored/<basename>-paths.yaml`. Each stub SHALL have `filter`, `input` (the fixture content), and `expected_output` set to the actual jq output (captured at generation time), plus version metadata.

#### Scenario: Paths extracted from demo.json
- **WHEN** `explore-paths.sh demo/demo.json` is run
- **THEN** the output YAML contains one scenario per distinct path including nested array element paths like `.orders[0].customer.name`

#### Scenario: Existing stubs not overwritten
- **WHEN** `skills/testing/scenarios/explored/<basename>-paths.yaml` already exists
- **THEN** the script appends only paths not already present in the file (matched by `filter` value) and prints `ADDED N new paths, skipped M existing`

### Requirement: Mark explored paths by coverage status
Each explored scenario stub SHALL include a `coverage` field set to `untested` at generation time. After the TUI runner executes a scenario successfully, it SHALL update the `coverage` field to `tested` in the YAML file.

#### Scenario: Coverage field initialized
- **WHEN** `explore-paths.sh` writes a new scenario stub
- **THEN** the stub has `coverage: untested`

#### Scenario: Coverage updated after passing run
- **WHEN** the runner executes a scenario with `coverage: untested` and it passes
- **THEN** the runner sets `coverage: tested` in the YAML file using `yq` in-place edit

### Requirement: Dependencies managed via mise
`explore-paths.sh` SHALL check that `jaq` is available via mise before running. `jq` is used optionally for golden-path validation; the script SHALL warn but not exit if absent.

#### Scenario: jaq checked at startup
- **WHEN** `jaq` is not on PATH
- **THEN** the script prints `ERROR: jaq not found. Run: mise install` and exits 1

#### Scenario: jq absent
- **WHEN** `jq` is not on PATH
- **THEN** the script prints `WARN: jq not found; golden-path comparison disabled` and continues
