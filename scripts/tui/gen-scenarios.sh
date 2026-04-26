#!/usr/bin/env bash
# gen-scenarios.sh — Generate jq test scenario YAML files for a named jq function.
#
# Usage:
#   gen-scenarios.sh <function-name>
#
# Produces:
#   test-scenarios/<function-name>.yaml             — evaluation scenarios
#   test-scenarios/intellisense/<function-name>-intellisense.yaml — TUI stubs
#
# Requires jaq (mise install). jq is used as a secondary validator if available.
#
# Filter/description pairs are separated by a tab character so that the
# jq pipe operator (|) can appear freely in filter strings.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCENARIOS_DIR="$REPO_ROOT/test-scenarios"
INTELLISENSE_DIR="$SCENARIOS_DIR/intellisense"

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
  echo "WARN: jq not found; divergence checking disabled"
fi

# ---------------------------------------------------------------------------
# Argument validation
# ---------------------------------------------------------------------------

if [ $# -ne 1 ]; then
  echo "Usage: gen-scenarios.sh <function-name>"
  echo "Example: gen-scenarios.sh select"
  exit 1
fi

FUNCTION_NAME="$1"

# ---------------------------------------------------------------------------
# Version capture
# ---------------------------------------------------------------------------

JQPP_VERSION="unknown"
JQPP_GIT_SHA="unknown"

if [ -f "$REPO_ROOT/target/debug/jqpp" ]; then
  JQPP_VERSION=$("$REPO_ROOT/target/debug/jqpp" --version 2>/dev/null | head -1 || echo "unknown")
else
  if command -v cargo >/dev/null 2>&1; then
    JQPP_VERSION=$(cargo metadata --manifest-path "$REPO_ROOT/Cargo.toml" \
      --no-deps --format-version 1 2>/dev/null \
      | jaq -r '.packages[0].version' 2>/dev/null || echo "unknown")
  fi
fi

if command -v git >/dev/null 2>&1; then
  JQPP_GIT_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
fi

TODAY=$(date +%Y-%m-%d)

echo "Generating scenarios for: $FUNCTION_NAME"
echo "  jqpp: $JQPP_VERSION ($JQPP_GIT_SHA)"
echo ""

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

# validate_scenario INPUT FILTER → prints jaq output on success, returns 1 on failure
validate_scenario() {
  local input="$1" filter="$2" output
  if output=$(echo "$input" | jaq "$filter" 2>/dev/null); then
    echo "$output"; return 0
  else
    return 1
  fi
}

# check_jq_diverges INPUT FILTER JAQ_OUTPUT → "true" or "false"
check_jq_diverges() {
  local input="$1" filter="$2" jaq_output="$3"
  if $JQ_AVAILABLE; then
    local jq_output
    jq_output=$(echo "$input" | jq "$filter" 2>/dev/null || echo "__jq_error__")
    [ "$jq_output" != "$jaq_output" ] && echo "true" && return
  fi
  echo "false"
}

# yaml_scalar VALUE → inline quote or literal block scalar
yaml_scalar() {
  local val="$1"
  if printf '%s' "$val" | grep -q $'\n'; then
    printf "|\n"; printf '%s\n' "$val" | sed 's/^/      /'
  else
    printf '"%s"\n' "$val"
  fi
}

# ---------------------------------------------------------------------------
# Inputs: tab-separated "VALUE<TAB>DESCRIPTION"
# ---------------------------------------------------------------------------
TAB=$'\t'

declare -a INPUTS=(
  "42${TAB}number input"
  "\"hello\"${TAB}string input"
  "[10, 55, 23, 80]${TAB}array of numbers"
  "[\"foo\",\"bar\",\"baz\"]${TAB}array of strings"
  "[{\"name\":\"Alice\",\"age\":30},{\"name\":\"Bob\",\"age\":17}]${TAB}array of objects"
  "{\"key\":\"value\",\"count\":5}${TAB}object input"
)

# ---------------------------------------------------------------------------
# Filters: tab-separated "FILTER<TAB>DESCRIPTION"
# (tab is safe — jq filters never contain literal tabs)
# ---------------------------------------------------------------------------

get_filters_for_function() {
  local fn="$1"
  case "$fn" in
    select)
      printf '. | select(. != null)\tidentity check\n'
      printf '.[] | select(. > 30)\tfilter numbers > 30\n'
      printf '.[] | select(.age >= 18)\tfilter objects by age\n'
      printf 'map(select(. > 0))\tmap select positive\n'
      ;;
    map)
      printf 'map(. * 2)\tdouble each element\n'
      printf 'map(.name)\textract name field\n'
      printf 'map(select(. > 10))\tfilter in map\n'
      ;;
    length)
      printf 'length\tstring or array length\n'
      printf '[.[] | length]\tlength of each element\n'
      ;;
    keys)
      printf 'keys\tobject keys\n'
      printf '[.[] | keys]\tkeys of each object\n'
      ;;
    has)
      printf 'has("name")\thas name field\n'
      printf 'has(0)\thas first element\n'
      ;;
    contains)
      printf 'contains(10)\tcontains number\n'
      printf 'contains("foo")\tcontains string\n'
      printf 'contains({"key":"value"})\tcontains object subset\n'
      ;;
    type)
      printf 'type\tget type\n'
      printf '[.[] | type]\ttype of each element\n'
      ;;
    to_entries)
      printf 'to_entries\tobject to entries\n'
      printf 'to_entries | map(.key)\textract keys via entries\n'
      ;;
    from_entries)
      printf '[{"key":"a","value":1}] | from_entries\tentries to object\n'
      ;;
    flatten)
      printf 'flatten\tflatten nested arrays\n'
      printf 'flatten(1)\tflatten one level\n'
      ;;
    unique)
      printf 'unique\tdeduplicate array\n'
      printf 'unique_by(.name)\tdeduplicate by field\n'
      ;;
    group_by)
      printf 'group_by(. > 30)\tgroup numbers by condition\n'
      printf 'group_by(.age)\tgroup objects by field\n'
      ;;
    sort_by)
      printf 'sort_by(.)\tsort by identity\n'
      printf 'sort_by(.age)\tsort objects by age field\n'
      ;;
    min|max)
      printf '.| %s\t%s of array\n' "$fn" "$fn"
      printf '[.[]|.age] | %s\t%s of field values\n' "$fn" "$fn"
      ;;
    add)
      printf 'add\tsum array\n'
      printf '[.[] | .count] | add\tsum field values\n'
      ;;
    any|all)
      printf '%s(. > 0)\t%s with condition\n' "$fn" "$fn"
      printf '%s(.[]; . > 10)\t%s on array elements\n' "$fn" "$fn"
      ;;
    recurse)
      printf 'recurse | numbers\trecurse to find numbers\n'
      ;;
    paths)
      printf '[paths]\tall paths\n'
      ;;
    *)
      printf '%s\tbasic application\n' "$fn"
      printf '.[] | %s\tapply to each element\n' "$fn"
      ;;
  esac
}

# ---------------------------------------------------------------------------
# Generate evaluation scenarios
# ---------------------------------------------------------------------------

OUTPUT_FILE="$SCENARIOS_DIR/${FUNCTION_NAME}.yaml"

SCENARIO_COUNT=0
SKIP_COUNT=0

cat > "$OUTPUT_FILE" << METAEOF
meta:
  created: "$TODAY"
  jqpp_version: "$JQPP_VERSION"
  jqpp_git_sha: "$JQPP_GIT_SHA"
  topic: "${FUNCTION_NAME}-function"
  description: "Evaluation scenarios for the ${FUNCTION_NAME}() function"

scenarios:
METAEOF

while IFS=$'\t' read -r filter filter_desc; do
  [ -z "$filter" ] && continue

  for input_entry in "${INPUTS[@]}"; do
    IFS=$'\t' read -r input input_desc <<< "$input_entry"

    actual_output=$(validate_scenario "$input" "$filter") || {
      echo "SKIP: invalid filter for input ($input_desc): $filter"
      SKIP_COUNT=$((SKIP_COUNT+1))
      continue
    }

    jq_diverges=$(check_jq_diverges "$input" "$filter" "$actual_output")

    scenario_id="${FUNCTION_NAME}-$(printf '%s' "$filter_desc" | tr ' ' '-' | tr -cd '[:alnum:]-' | tr '[:upper:]' '[:lower:]')-$(printf '%s' "$input_desc" | tr ' ' '-' | tr -cd '[:alnum:]-' | tr '[:upper:]' '[:lower:]')"
    scenario_id="${scenario_id:0:80}"

    expected_yaml=$(yaml_scalar "$actual_output")

    # Single-quote the filter for YAML; escape any single quotes inside it
    filter_yaml="'$(printf '%s' "$filter" | sed "s/'/'\\''/g")'"
    input_yaml="'$(printf '%s' "$input" | sed "s/'/'\\''/g")'"

    cat >> "$OUTPUT_FILE" << SCENEOF
  - id: $scenario_id
    description: "$filter_desc with $input_desc"
    input: $input_yaml
    filter: $filter_yaml
    expected_output: $expected_yaml
    jq_diverges: $jq_diverges
    tags: [$FUNCTION_NAME]

SCENEOF

    SCENARIO_COUNT=$((SCENARIO_COUNT+1))
    echo "  wrote: $scenario_id"
  done
done < <(get_filters_for_function "$FUNCTION_NAME")

echo ""
echo "Wrote $SCENARIO_COUNT scenarios to $OUTPUT_FILE  (skipped $SKIP_COUNT invalid)"

# ---------------------------------------------------------------------------
# Generate intellisense scenario stub
# ---------------------------------------------------------------------------

INTELLISENSE_FILE="$INTELLISENSE_DIR/${FUNCTION_NAME}-intellisense.yaml"

ACTIONS_YAML=""
for ((i=0; i<${#FUNCTION_NAME}; i++)); do
  char="${FUNCTION_NAME:$i:1}"
  ACTIONS_YAML+="    - type: \"$char\""$'\n'
done
ACTIONS_YAML+="    - type: \"(\""$'\n'

cat > "$INTELLISENSE_FILE" << IEOF
meta:
  created: "$TODAY"
  jqpp_version: "$JQPP_VERSION"
  jqpp_git_sha: "$JQPP_GIT_SHA"
  topic: "${FUNCTION_NAME}-intellisense"
  description: "TUI intellisense scenario stubs for ${FUNCTION_NAME}()"

scenarios:
  - id: ${FUNCTION_NAME}-open-suggests
    description: "After typing ${FUNCTION_NAME}( suggestions appear"
    input_file: "demo/demo.json"
    asserts_with: [tui_suggestion_box_visible, tui_extract_suggestions]
    actions:
$ACTIONS_YAML    assertions:
      - suggestions_visible: true
      - suggestions_contain: []
      - suggestions_not_contain: []
IEOF

echo "Wrote intellisense stub to $INTELLISENSE_FILE"
echo ""
echo "Next steps:"
echo "  1. Review $OUTPUT_FILE and remove any nonsensical scenarios"
echo "  2. Fill in suggestions_contain in $INTELLISENSE_FILE"
echo "  3. Run: scripts/tui/run-scenarios.sh $OUTPUT_FILE"
