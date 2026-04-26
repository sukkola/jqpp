---
name: jqpp-scenario-gen
description: >
  Generate structured jq test scenarios for jqpp intellisense and query evaluation.
  Produces reproducible scenario files with jq filters, expected outputs, and the
  jqpp version they were created against. Use when building regression suites, exploring
  edge cases, or documenting expected behaviour for a specific app version.
user-invocable: true
allowed-tools: Bash, Read, Write, Glob, Grep
---

# jqpp Scenario Generation Skill

Generate and store structured test scenarios for jqpp query evaluation and intellisense.

## Version capture

Every scenario file **must** record the jqpp version it was created against. Always capture
the version before generating scenarios and embed it in the scenario file header.

```bash
# Capture version at scenario generation time
JQPP_VERSION=$(target/debug/jqpp --version 2>/dev/null || cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "jqpp") | .version')
GIT_SHA=$(git rev-parse --short HEAD)
echo "jqpp $JQPP_VERSION ($GIT_SHA)"
```

Store both the semver and the git SHA so scenarios can be reproduced against a specific
build even if the version number hasn't changed.

## Scenario file format

Store scenarios as YAML in `test-scenarios/`. Each file covers one topic or feature area.
Intellisense scenarios go in `test-scenarios/intellisense/`.

All scenarios MUST be in the `scenarios:` list. Do NOT use `intellisense_scenarios:`.

### Evaluation Scenario Example

```yaml
meta:
  created: "2026-04-24"
  jqpp_version: "0.1.0"
  jqpp_git_sha: "8020921"
  topic: "select-function"
  description: "Scenarios covering select() filter behaviour"

scenarios:
  - id: select-number-gt
    description: "Filter array of numbers > 30"
    input: '[10, 55, 23, 80]'
    filter: ".[] | select(. > 30)"
    expected_output: |
      55
      80
    tags: [select, number, array]
    status: active
```

### TUI Intellisense Scenario Example

```yaml
meta:
  created: "2026-04-24"
  jqpp_version: "0.1.0"
  jqpp_git_sha: "8020921"
  topic: "select-intellisense"
  description: "Intellisense behaviour for select() function"

scenarios:
  - id: select-dot-suggests-fields
    description: "After .orders[]|select(. suggestions show field names"
    input_file: "demo/demo.json"
    asserts_with: [tui_suggestion_box_visible, tui_extract_suggestions]
    actions:
      - type: ".orders[] | select(."
      - wait_stable: true
    assertions:
      - suggestions_visible: true
      - suggestions_contain: ["order_id", "customer", "status"]
    status: active
```

## Validation (MANDATORY)

Before completing a scenario generation task, you MUST validate the file structure.

```bash
# Validate structure
mise run tui-validate-scenario test-scenarios/your-file.yaml

# Verify listing (also checks basic YAML parsing)
mise run tui-run-scenarios --list test-scenarios/your-file.yaml
```

## Quick scenario generation workflow

1. Capture JQPP_VERSION and GIT_SHA.
2. Define scenarios in a YAML file under `test-scenarios/` (use `intellisense/` subdirectory for TUI tests).
3. **MANDATORY**: Run `mise run tui-validate-scenario <path>` to ensure structural correctness.
4. Run evaluation scenarios to verify logic: `mise run tui-run-scenarios <path>`.
5. For TUI scenarios, verify they are listed: `mise run tui-run-scenarios --list <path>`.
