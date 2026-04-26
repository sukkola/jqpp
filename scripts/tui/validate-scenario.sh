#!/usr/bin/env bash
# validate-scenario.sh — Validate a jqpp scenario YAML file against structural rules.
# Usage: scripts/tui/validate-scenario.sh <file.yaml>

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
YAML2JSON="$SCRIPT_DIR/lib/yaml2json.sh"

if [ $# -ne 1 ]; then
  echo "Usage: $0 <file.yaml>" >&2
  exit 1
fi

FILE="$1"
JSON=$( "$YAML2JSON" "$FILE" )

# 1. Validate meta block
echo "Checking meta block..."
echo "$JSON" | jq -e '.meta | has("created") and has("jqpp_version") and has("jqpp_git_sha") and has("topic")' >/dev/null || {
  echo "ERROR: Missing or invalid 'meta' block in $FILE" >&2
  echo "Expected fields: created, jqpp_version, jqpp_git_sha, topic" >&2
  exit 1
}

# 2. Validate scenarios list
echo "Checking scenarios list..."
echo "$JSON" | jq -e '.scenarios | type == "array"' >/dev/null || {
  echo "ERROR: 'scenarios' must be an array in $FILE" >&2
  exit 1
}

# 3. Validate each scenario
SCENARIO_COUNT=$(echo "$JSON" | jq '.scenarios | length')
echo "Validating $SCENARIO_COUNT scenarios..."

for i in $(seq 0 $((SCENARIO_COUNT - 1))); do
  SCENARIO=$(echo "$JSON" | jq ".scenarios[$i]")
  ID=$(echo "$SCENARIO" | jq -r '.id')
  
  if [ "$ID" == "null" ]; then
    echo "ERROR: Scenario at index $i is missing an 'id'" >&2
    exit 1
  fi

  # Common fields
  echo "$SCENARIO" | jq -e 'has("description") and has("status")' >/dev/null || {
    echo "ERROR: Scenario '$ID' missing 'description' or 'status'" >&2
    exit 1
  }

  # Determine type (Evaluation vs TUI)
  if echo "$SCENARIO" | jq -e 'has("filter")' >/dev/null; then
    # Evaluation scenario
    echo "  [$ID] Validating evaluation scenario..."
    echo "$SCENARIO" | jq -e 'has("input") and (has("expected_output") or has("expected_error"))' >/dev/null || {
      echo "ERROR: Evaluation scenario '$ID' missing 'input' or 'expected_output'/'expected_error'" >&2
      exit 1
    }
  elif echo "$SCENARIO" | jq -e 'has("actions")' >/dev/null; then
    # TUI scenario
    echo "  [$ID] Validating TUI scenario..."
    echo "$SCENARIO" | jq -e 'has("input_file") and (.actions | type == "array") and (.assertions | type == "array")' >/dev/null || {
      echo "ERROR: TUI scenario '$ID' missing 'input_file', 'actions' (array), or 'assertions' (array)" >&2
      exit 1
    }
  else
    echo "ERROR: Scenario '$ID' must have either 'filter' (evaluation) or 'actions' (TUI)" >&2
    exit 1
  fi
done

echo "OK: $FILE is structurally valid."
