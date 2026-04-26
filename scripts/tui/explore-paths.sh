#!/usr/bin/env bash
# explore-paths.sh — Generate scenario stubs by enumerating jq paths in a JSON fixture.
#
# Usage:
#   explore-paths.sh <fixture.json>
#
# Produces:
#   test-scenarios/explored/<basename>-paths.yaml
#
# Appends only paths not already present in the output file.
# Requires jaq (mise install). jq is used as a golden-path validator if available.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCENARIOS_DIR="$REPO_ROOT/test-scenarios"
EXPLORED_DIR="$SCENARIOS_DIR/explored"

# ---------------------------------------------------------------------------
# Dependency checks
# ---------------------------------------------------------------------------

if ! command -v jaq >/dev/null 2>&1; then
  echo "ERROR: jaq not found. Run: mise install"
  exit 1
fi

JQ_AVAILABLE=false
if command -v jq >/dev/null 2>&1; then
  JQ_AVAILABLE=true
else
  echo "WARN: jq not found; golden-path comparison disabled"
fi

# ---------------------------------------------------------------------------
# Argument validation
# ---------------------------------------------------------------------------

if [ $# -ne 1 ]; then
  echo "Usage: explore-paths.sh <fixture.json>"
  echo "Example: explore-paths.sh demo/demo.json"
  exit 1
fi

FIXTURE="$1"

# Resolve relative to repo root if not absolute
if [[ "$FIXTURE" != /* ]]; then
  FIXTURE="$REPO_ROOT/$FIXTURE"
fi

if [ ! -f "$FIXTURE" ]; then
  echo "ERROR: fixture file not found: $FIXTURE"
  exit 1
fi

# Validate it is valid JSON
if ! jaq '.' < "$FIXTURE" > /dev/null 2>&1; then
  echo "ERROR: invalid JSON in fixture: $FIXTURE"
  exit 1
fi

BASENAME=$(basename "$FIXTURE" .json)
OUTPUT_FILE="$EXPLORED_DIR/${BASENAME}-paths.yaml"
mkdir -p "$EXPLORED_DIR"

# ---------------------------------------------------------------------------
# Version capture
# ---------------------------------------------------------------------------

JQPP_VERSION="unknown"
JQPP_GIT_SHA="unknown"

if [ -f "$REPO_ROOT/target/debug/jqpp" ]; then
  JQPP_VERSION=$("$REPO_ROOT/target/debug/jqpp" --version 2>/dev/null | head -1 || echo "unknown")
elif command -v cargo >/dev/null 2>&1; then
  JQPP_VERSION=$(cargo metadata --manifest-path "$REPO_ROOT/Cargo.toml" \
    --no-deps --format-version 1 2>/dev/null \
    | jaq -r '.packages[0].version' 2>/dev/null || echo "unknown")
fi

if command -v git >/dev/null 2>&1; then
  JQPP_GIT_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
fi

TODAY=$(date +%Y-%m-%d)

# ---------------------------------------------------------------------------
# Load fixture content for embedding in scenarios
# ---------------------------------------------------------------------------

FIXTURE_CONTENT=$(cat "$FIXTURE")

# ---------------------------------------------------------------------------
# Enumerate leaf paths
# ---------------------------------------------------------------------------

# jaq '[path(..)]' enumerates all paths including intermediate nodes.
# We convert each path array to a jq filter string like .foo[0].bar
paths_to_filters() {
  # Each path is a JSON array of keys/indices, e.g. ["orders",0,"customer","name"]
  # Convert to a dotted filter string.
  jaq -r '[path(..)] | .[] |
    reduce .[] as $seg (
      "";
      if ($seg | type) == "number"
      then . + "[" + ($seg | tostring) + "]"
      else . + "." + $seg
      end
    )' < "$FIXTURE"
}

echo "Enumerating paths in $FIXTURE..."
# Collect all path filters into a temp file (avoids mapfile / bash 4 requirement)
TMP_PATHS=$(mktemp)
paths_to_filters > "$TMP_PATHS"
TOTAL_PATHS=$(wc -l < "$TMP_PATHS" | tr -d ' ')
echo "  found $TOTAL_PATHS paths (including intermediate nodes)"

# ---------------------------------------------------------------------------
# Load existing filters to deduplicate
# ---------------------------------------------------------------------------

EXISTING_FILTERS=""
if [ -f "$OUTPUT_FILE" ]; then
  EXISTING_FILTERS=$(python3 - "$OUTPUT_FILE" <<'PYEOF'
import sys, yaml
with open(sys.argv[1]) as f:
    data = yaml.safe_load(f) or {}
for s in data.get("scenarios", []) or []:
    if s and "filter" in s:
        print(s["filter"])
PYEOF
)
fi

# ---------------------------------------------------------------------------
# Write output file header if it doesn't exist
# ---------------------------------------------------------------------------

if [ ! -f "$OUTPUT_FILE" ]; then
  cat > "$OUTPUT_FILE" << METAEOF
meta:
  created: "$TODAY"
  jqpp_version: "$JQPP_VERSION"
  jqpp_git_sha: "$JQPP_GIT_SHA"
  topic: "${BASENAME}-paths"
  description: "Path exploration scenarios for fixture: $BASENAME.json"

scenarios:
METAEOF
fi

# ---------------------------------------------------------------------------
# Generate stubs for new paths
# ---------------------------------------------------------------------------

ADDED=0
SKIPPED=0

while IFS= read -r filter; do
  [ -z "$filter" ] && continue

  # Deduplicate: skip if filter already in output file
  if echo "$EXISTING_FILTERS" | grep -qF "$filter"; then
    SKIPPED=$((SKIPPED+1))
    continue
  fi

  # Run filter with jaq to get expected output
  actual_output=$(echo "$FIXTURE_CONTENT" | jaq "$filter" 2>/dev/null) || {
    SKIPPED=$((SKIPPED+1))
    continue
  }

  jq_diverges="false"
  if $JQ_AVAILABLE; then
    jq_output=$(echo "$FIXTURE_CONTENT" | jq "$filter" 2>/dev/null || echo "__jq_error__")
    if [ "$jq_output" != "$actual_output" ]; then
      jq_diverges="true"
    fi
  fi

  # Build scenario id from filter (sanitize)
  scenario_id="${BASENAME}-$(echo "$filter" | tr -cd '[:alnum:]._-' | tr '.' '-' | sed 's/--*/-/g' | sed 's/^-//;s/-$//')"
  scenario_id="${scenario_id:0:80}"

  # Encode expected_output as YAML scalar.
  # Use literal block scalar (|) only when the value contains real newlines.
  filter_yaml="'$(printf '%s' "$filter" | sed "s/'/'\\''/g")'"
  input_yaml="'$(printf '%s' "$FIXTURE_CONTENT" | tr -d '\n' | sed "s/'/'\\''/g")'"

  if printf '%s' "$actual_output" | grep -q $'\n'; then
    # Multi-line: write literal block scalar then indented content
    cat >> "$OUTPUT_FILE" <<STUBEOF
  - id: $scenario_id
    description: "Path exploration: $filter"
    input: $input_yaml
    filter: $filter_yaml
    expected_output: |
$(printf '%s\n' "$actual_output" | sed 's/^/      /')
    jq_diverges: $jq_diverges
    coverage: untested
    asserts_with: [tui_read_query]

STUBEOF
  else
    cat >> "$OUTPUT_FILE" <<STUBEOF
  - id: $scenario_id
    description: "Path exploration: $filter"
    input: $input_yaml
    filter: $filter_yaml
    expected_output: "$actual_output"
    jq_diverges: $jq_diverges
    coverage: untested
    asserts_with: [tui_read_query]

STUBEOF
  fi

  ADDED=$((ADDED+1))
done < "$TMP_PATHS"
rm -f "$TMP_PATHS"

echo "ADDED $ADDED new paths, skipped $SKIPPED existing"
echo "Output: $OUTPUT_FILE"
