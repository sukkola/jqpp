## Context

jqpp has two skills in progress — `jqpp-tui-test` and `jqpp-scenario-gen` — that overlap significantly: both need to launch jqpp via agent-tui, parse screenshots, store scenario data, and compare outputs across versions. Currently they have no shared code, no shared directory structure, and no standalone runner. All testing is ad-hoc and requires an LLM session.

The skills live in `skills/` (repo-native) and are symlinked into `.claude/skills/`. The test framework must fit this structure: shell scripts callable from both skills and from CI, with scenario data stored alongside the scripts so the LLM can inspect them when needed without fetching external state.

## Goals / Non-Goals

**Goals:**
- Single shared directory layout (`skills/testing/`) used by both `jqpp-tui-test` and `jqpp-scenario-gen`
- Standalone bash runner: `scripts/tui/run-scenarios.sh <file.yaml>` exits 0/1 with pass/fail summary
- Reusable screenshot-parsing library (`scripts/tui/lib/screenshot.sh`) sourced by runners and skill scripts
- Canonical YAML scenario format covering: jq evaluation, TUI keystroke sequences, suggestion box expectations
- Version tagging in every scenario file (semver + git SHA at generation time)
- Scenario generation workflow for named jq functions (docs → fixtures → YAML stubs)
- Query-path exploration workflow for arbitrary JSON fixtures

**Non-Goals:**
- CI integration (wiring into GitHub Actions) — separate change
- LLM-driven test repair or auto-fix — scripts report failures, LLM acts if invoked
- Modifying jqpp Rust source
- Replacing agent-tui with a different automation backend

## Decisions

### D1 — YAML as scenario format

**Decision**: YAML, not shell scripts or JSON.

**Rationale**: Shell scripts are executable but not machine-readable for diff/compare workflows. JSON is readable but not human-friendly for multi-line `expected_output` blocks. YAML handles multi-line values cleanly, is `yq`-parseable, and can be read by both LLMs and scripts. `jq` cannot parse YAML natively but the runner uses `yq` to convert before processing.

**Alternatives**: shell scripts (chosen earlier in jqpp-tui-test SKILL.md) — too opaque for automated comparison; JSON — multi-line output is ugly.

### D2 — Shared `skills/testing/` directory, scripts in `scripts/tui/`

**Decision**: Scenario data in `skills/testing/scenarios/` and `skills/testing/baselines/`. Runner and library scripts in `scripts/tui/`.

**Rationale**: Skills directories are LLM-facing (instructions + scenarios). `scripts/` is CI/shell-facing (executables). Keeping them separate prevents the LLM from treating runner scripts as scenario definitions. The existing skill symlink pattern means `skills/testing/` is git-tracked and both skills can reference it with a relative path.

**Alternatives**: Everything under `.claude/skills/` — not git-tracked cleanly; everything under `tests/` — correct for project tests but the scenarios are also LLM context, so skills/ is the right home.

### D3 — YAML → JSON → jaq/jq pipeline instead of yq

**Decision**: Scenario files are stored as YAML for human readability, but all processing uses a YAML-to-JSON conversion step piped into `jaq` or `jq`. No `yq` dependency. jqpp itself supports both JSON and YAML input, so the same conversion path used by the app is available in scripts.

**Rationale**: `yq` implements a much smaller surface of the jq filter language than `jq` itself does. Using `yq` for anything beyond trivial field reads risks silent mis-evaluation. The YAML → JSON → `jaq` pipeline is more correct, uses tools already required by the project, and avoids an extra dependency. A small conversion helper (`scripts/tui/lib/yaml2json.sh` or a one-liner using Python's `json`/`yaml` stdlib or the system `ruby -ryaml -rjson`) handles the conversion.

**Alternatives**: `yq` — less filter surface, divergent behaviour on complex expressions; store scenarios as JSON — loses multi-line readability.

### D3a — jaq as primary runner, jq as golden-path validator

**Decision**: `jaq` (the Rust implementation jqpp is built on) is the primary tool for running evaluation scenarios. `jq` (the C reference implementation) is used as a secondary validator when a scenario is flagged as divergent or needs confirmation that a filter is canonical jq-compatible.

**Rationale**: jqpp's intellisense is built around jaq's semantics. Testing with `jaq` ensures scenarios match what the app actually evaluates. When `jaq` and `jq` produce different output, the divergence itself is valuable information — it surfaces jaq compatibility gaps that jqpp users would encounter. `jq` is not managed via mise (it's a system package) to avoid duplicating what developers already have; `jaq` is provisioned via mise since it is project-specific.

**Alternatives**: Use only `jq` — misses jaq-specific behaviour; use only `jaq` — can't identify divergence.

### D4 — Screenshot library as sourced bash functions, not a binary

**Decision**: `scripts/tui/lib/screenshot.sh` exposes functions (`tui_extract_suggestions`, `tui_read_query`, `tui_suggestion_box_visible`) that callers `source`.

**Rationale**: No compilation, no PATH management, easy to read and extend by LLM. Functions operate on a `$SCREEN` variable passed by the caller, so they are unit-testable without a running agent-tui session.

**Alternatives**: A Rust binary for parsing — correct but overkill for text extraction; Python script — extra dependency.

### D5 — Scenario generation via `jq` + web fetch, not LLM generation alone

**Decision**: The scenario-gen workflow fetches jqlang.org/manual content, extracts examples with `grep`/`sed`, validates them with `jq`, and writes YAML stubs. LLM fills gaps but scripts do the mechanical parts.

**Rationale**: Keeps generated test data reproducible and version-anchored. An LLM-only workflow would produce non-deterministic scenario files that cannot be reliably compared across runs.

## Risks / Trade-offs

- `yq` version sensitivity → pin to `yq` v4 syntax in all scripts; document in README
- agent-tui `--strip-ansi` output format may change → screenshot lib abstracts the parsing; update lib when format changes
- YAML multi-line `expected_output` whitespace handling → runner must normalize trailing whitespace before comparison (use `sed 's/[[:space:]]*$//'`)
- Scenario files with wrong jqpp_version metadata → scripts validate version field exists but cannot enforce correctness; generation workflow must always capture version before writing
- `jqlang.org` may be unreachable in CI → scenario-gen is an offline-capable workflow after initial generation; runner scripts never fetch external URLs

## Open Questions

- Should the runner also validate suggestion box pixel-column position, or only content? Column validation is fragile if terminal width changes. (Proposed: content only for now, position as optional assertion.)
- Should `skills/testing/baselines/` store full screen text or just suggestion-box extracts? (Proposed: suggestion-box extracts only — full screens are too fragile to terminal size.)
