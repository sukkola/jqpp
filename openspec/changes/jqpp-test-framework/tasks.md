## 1. mise dependency management

- [x] 1.1 Confirm `aqua:01mf02/jaq` is the correct aqua package name for jaq (`mise registry | grep jaq`) and update `mise.toml` entry if needed
- [x] 1.2 Add `agent-tui` tool entry to `mise.toml` if installable via mise (otherwise keep existing comment documenting manual install)
- [x] 1.3 Verify `mise install` installs `jaq` and it is on PATH in the mise environment

## 2. Shared directory layout

- [x] 2.1 Create `test-scenarios/` directory with a `README.md` listing the layout
- [x] 2.2 Create `test-scenarios/intellisense/` and `test-scenarios/explored/` subdirectories (with `.gitkeep`)
- [x] 2.3 Create `test-scenarios/baselines/` directory (with `.gitkeep`)
- [x] 2.4 Create `scripts/tui/` directory
- [x] 2.5 Create `scripts/tui/lib/` directory

## 3. YAML-to-JSON helper and screenshot parsing library

- [x] 3.1 Write `scripts/tui/lib/yaml2json.sh` — Python one-liner wrapper that converts a YAML file to JSON on stdout (requires Python 3 with `yaml` stdlib, available on all target platforms)
- [x] 3.2 Write `scripts/tui/lib/screenshot.sh` with `tui_suggestion_box_visible SCREEN` function
- [x] 3.3 Add `tui_extract_suggestions SCREEN` function to the library
- [x] 3.4 Add `tui_read_query SCREEN` function to the library
- [x] 3.5 Write inline unit tests for the library functions using heredoc screen fixtures (no agent-tui required)

## 4. Standalone TUI test runner

- [x] 4.1 Write `scripts/tui/run-scenarios.sh` skeleton: argument parsing (file or directory, `--update-baselines` flag), dependency check for `jaq`/`agent-tui`, help text
- [x] 4.2 Implement YAML→JSON→jaq pipeline: call `yaml2json` then `jaq` to extract meta and scenario arrays
- [x] 4.3 Implement evaluation scenario execution: run `jaq "$filter"`, compare output, check `jq_diverges` flag, report `PASS`/`FAIL`/divergence warning
- [x] 4.4 Implement `jq` secondary validation: when `jq` is on PATH, run the same filter with `jq` and compare; set `jq_diverges` warning if outputs differ and flag is not set
- [x] 4.5 Implement version metadata validation: capture current jqpp version, warn on mismatch with scenario `meta.jqpp_version`
- [x] 4.6 Implement TUI scenario execution: agent-tui launch, wait for Query bar, execute action sequence, capture screenshot
- [x] 4.7 Implement `asserts_with` dispatch: call only the screenshot lib functions listed in the scenario's `asserts_with` field; default to all three with a warning if absent
- [x] 4.8 Implement baseline writing: write raw screenshot to `baselines/<id>-raw.txt` and function-specific extracts to `baselines/<id>-<fn>.txt`; overwrite if `--update-baselines`
- [x] 4.9 Implement assertion evaluation: `suggestions_visible`, `suggestions_contain`, `suggestions_not_contain`, `query_contains`
- [x] 4.10 Implement session cleanup (kill) on assertion failure or timeout
- [x] 4.11 Implement `regression` and `skip` status handling
- [x] 4.12 Implement summary output: `Results: N passed, N failed, N expected-fail`
- [x] 4.13 Make `run-scenarios.sh` executable and add shebang

## 5. Scenario generation script

- [x] 5.1 Write `scripts/tui/gen-scenarios.sh <function-name>` skeleton: dependency check (`jaq` required, `jq` optional), argument validation, version capture
- [x] 5.2 Implement evaluation scenario generation: produce input/filter/expected_output triples for number, string, array, object input types, validate each with `jaq`; when `jq` is available run it too and set `jq_diverges: true` if outputs differ
- [x] 5.3 Implement YAML file writing with `meta` block and scenario list using heredoc/printf (no yq)
- [x] 5.4 Implement intellisense scenario stub generation: write character-by-character `type` actions for filter prefix up to `(`, set `asserts_with: [tui_suggestion_box_visible, tui_extract_suggestions]` by default
- [x] 5.5 Make `gen-scenarios.sh` executable

## 6. Query path exploration script

- [x] 6.1 Write `scripts/tui/explore-paths.sh <fixture.json>` skeleton: dependency check (`jaq` required, `jq` optional), fixture validation
- [x] 6.2 Implement path enumeration using `jaq '[path(..)]'` and filter to leaf paths
- [x] 6.3 Implement per-path scenario stub generation: run each path as a filter with `jaq`, capture output, set `jq_diverges` if `jq` differs, write YAML stub with `coverage: untested` and `asserts_with: [tui_read_query]`
- [x] 6.4 Implement deduplication: skip paths already present in the output file (match by `filter` value using `jaq`)
- [x] 6.5 Make `explore-paths.sh` executable

## 7. Skill updates

- [x] 7.1 Update `skills/jqpp-tui-test/SKILL.md` to reference shared paths (`test-scenarios/`, `scripts/tui/lib/screenshot.sh`) instead of inline grep patterns
- [x] 7.2 Update `skills/jqpp-scenario-gen/SKILL.md` to reference `gen-scenarios.sh` for scenario file creation and shared `test-scenarios/` for storage
- [x] 7.3 Add `test-scenarios/README.md` documenting directory layout, scenario YAML format, and how to run `run-scenarios.sh`

## 8. Seed scenario files

- [x] 8.1 Run `gen-scenarios.sh select` and review generated `test-scenarios/select.yaml`
- [x] 8.2 Run `explore-paths.sh demo/demo.json` and review `test-scenarios/explored/demo-paths.yaml`
- [x] 8.3 Manually add TUI intellisense scenarios to `test-scenarios/intellisense/` covering: dot-open suggestions, Tab accept, Enter accept+evaluate, Esc close
- [ ] 8.4 Run `run-scenarios.sh test-scenarios/` end-to-end and confirm all seed scenarios pass
