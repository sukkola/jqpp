# test-scenarios

Structured YAML test scenarios for jqpp — both jq evaluation tests and TUI intellisense tests.

## Directory layout

```
test-scenarios/
├── README.md                        # this file
├── <function>.yaml                  # evaluation scenarios per jq function
├── intellisense/
│   └── <function>-intellisense.yaml # TUI keystroke + suggestion assertions
├── explored/
│   └── <fixture>-paths.yaml        # auto-generated from explore-paths.sh
└── baselines/
    ├── <scenario-id>-raw.txt        # raw --strip-ansi screenshot after TUI run
    └── <scenario-id>-<fn>.txt      # output of a specific screenshot lib function
```

## Running scenarios

```bash
# Run a single file
mise run tui-run-scenarios test-scenarios/select.yaml

# Run all files recursively
mise run tui-run-scenarios test-scenarios/

# Run only one specific scenario (substring match on id)
mise run tui-run-scenarios --scenario dot-tab test-scenarios/intellisense/dot-open.yaml

# Filter by scenario query prefix and function usage
mise run tui-run-scenarios --query-starts-with ".orders" --query-function select test-scenarios/

# Preview which scenarios the filters would match (no execution)
mise run tui-run-scenarios --list --query-function select test-scenarios/

# Optional listing filters by scenario metadata
mise run tui-run-scenarios --list --created-before 2026-04-25 --created-with-version "0.1.0" test-scenarios/

# Fill in ~ placeholder assertions by running the TUI and capturing actual values
mise run tui-run-scenarios-capture test-scenarios/intellisense/dot-open.yaml

# Update baselines after an intentional behaviour change
mise run tui-run-scenarios-update-baselines test-scenarios/select.yaml
```

Exits 0 if all non-regression scenarios pass. Tooling is managed in `mise.toml`
(`jaq`, `jq`, `agent-tui`) — run `mise install` before running scenarios.

### Filling in ~ placeholders

TUI scenarios with `query_equals: ~` cannot pass until the expected value is filled in.
Use `--capture-actuals` to discover the real value from the running app:

```
$ mise run tui-run-scenarios-capture test-scenarios/intellisense/dot-open.yaml

CAPTURE: dot-tab-accepts-suggestion
  Suggested assertions (paste into YAML replacing ~ values):
      - query_equals: ".orders"
```

Copy the printed value into the scenario YAML, then run normally to assert.

## Generating new scenarios

```bash
# Generate evaluation + intellisense stubs for a jq function
mise run tui-gen-scenarios select

# Generate path-exploration stubs from a JSON fixture
mise run tui-explore-paths demo/demo.json

# Build jqpp and launch it with the input/query from a scenario
mise run tui-launch-scenario test-scenarios/intellisense/dot-open.yaml --id dot-open-suggests-fields

# Launch by id across all scenarios (fails if id is missing or duplicated)
mise run tui-launch-scenario --id dot-open-suggests-fields
```

## Scenario YAML format

Every scenario file must have a `meta` block and a `scenarios` list.

### meta block (required in every file)

```yaml
meta:
  created: "2026-04-24"
  jqpp_version: "0.3.1"     # from target/debug/jqpp --version
  jqpp_git_sha: "8f24adc"   # from git rev-parse --short HEAD
  topic: "select-function"
  description: "Scenarios covering select() filter behaviour"
```

### Evaluation scenario

Tests a jq filter against a known input/output pair. Runs with `jaq` (no TUI needed).

```yaml
scenarios:
  - id: select-number-gt
    description: "Filter array of numbers > 30"
    input: '[10, 55, 23, 80]'
    filter: ".[] | select(. > 30)"
    expected_output: |
      55
      80
    tags: [select, number, array]
    jq_diverges: false     # set true if jaq and jq produce different output
    status: active         # active | regression | skip
```

### TUI intellisense scenario

Tests what the suggestion box shows after a keystroke sequence. Requires agent-tui.

```yaml
scenarios:
  - id: select-dot-suggests-fields
    description: "After .orders[]|select(. suggestions show field names"
    input_file: "demo/demo.json"   # relative to repo root
    asserts_with: [tui_suggestion_box_visible, tui_extract_suggestions]
    actions:
      - type: "."
      - type: "o"
      - press: Tab
      - wait_stable: true
    assertions:
      - suggestions_visible: true
      - suggestions_contain: ["order_id", "customer", "status"]
      - suggestions_not_contain: ["metadata"]
      - query_contains: ".orders"
    status: active
```

#### Action types

| Key | Value | Meaning |
|-----|-------|---------|
| `type` | string | Type characters one by one (triggers intellisense) |
| `press` | key name | Press a single key: `Tab`, `BackTab`, `Enter`, `Escape`, `ctrl+c` |
| `wait_stable` | true | Wait for screen to stop changing |

#### Assertion types

| Key | Value | Meaning |
|-----|-------|---------|
| `suggestions_visible` | bool | Suggestion box present/absent |
| `suggestions_contain` | list of strings | Each string must appear in suggestion list |
| `suggestions_not_contain` | list of strings | None of these strings may appear |
| `query_contains` | string | Query bar text must contain this substring |
| `query_equals` | string or `~` | Query bar text must exactly equal this value; `~` (null) is a placeholder — use `--capture-actuals` to fill in |

#### asserts_with

List the screenshot lib functions relevant to this scenario:
- `tui_suggestion_box_visible` — box presence check
- `tui_extract_suggestions` — suggestion label extraction
- `tui_read_query` — query bar text

If omitted, the runner calls all three and warns.

### Regression / skip status

```yaml
    status: regression   # failure is EXPECTED FAIL, pass is UNEXPECTED PASS
    status: skip         # scenario is not run
```

### Path-exploration scenario (from explore-paths.sh)

Same as evaluation scenario but includes `coverage` field:

```yaml
  - id: demo-orders-0-customer-name
    filter: ".orders[0].customer.name"
    input: '{ ... }'
    expected_output: '"Alice Korhonen"'
    coverage: untested     # updated to "tested" after a passing run
    asserts_with: [tui_read_query]
```
