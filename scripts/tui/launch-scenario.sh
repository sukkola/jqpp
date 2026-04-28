#!/usr/bin/env bash
# launch-scenario.sh — Build jqpp and run it with scenario input.
#
# Usage:
#   launch-scenario.sh [--id <scenario-id>] [--no-query] [--print-only] [scenario.yaml | scenario-dir]
#
# If no path is provided, defaults to `test-scenarios/`.
# - path is a file: uses that file; default scenario is first item unless --id is given
# - path is a directory + --id: searches YAML files for exact id match

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LIB_DIR="$SCRIPT_DIR/lib"

SCENARIO_PATH=""
SCENARIO_FILE=""
SCENARIO_ID=""
USE_QUERY=true
PRINT_ONLY=false

usage() {
  echo "Usage: launch-scenario.sh [--id <scenario-id>] [--no-query] [--print-only] [scenario.yaml | scenario-dir]"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --id) SCENARIO_ID="$2"; shift 2 ;;
    --no-query) USE_QUERY=false; shift ;;
    --print-only) PRINT_ONLY=true; shift ;;
    -h|--help) usage; exit 0 ;;
    -*) echo "ERROR: unknown option: $1"; usage; exit 1 ;;
    *) SCENARIO_PATH="$1"; shift ;;
  esac
done

if [ -z "$SCENARIO_PATH" ]; then
  SCENARIO_PATH="test-scenarios/"
fi

if [[ "$SCENARIO_PATH" != /* ]]; then
  SCENARIO_PATH="$REPO_ROOT/$SCENARIO_PATH"
fi

if [ ! -e "$SCENARIO_PATH" ]; then
  echo "ERROR: scenario path not found: $SCENARIO_PATH"
  exit 1
fi

if ! command -v jaq >/dev/null 2>&1; then
  echo "ERROR: jaq not found. Run: mise install"
  exit 1
fi

if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: rg not found. Run: mise install"
  exit 1
fi

resolve_scenario_file_from_dir() {
  local scenario_dir="$1"
  local scenario_id="$2"
  local -a candidates=()
  local f
  while IFS= read -r f; do
    candidates+=("$f")
  done < <(rg --files "$scenario_dir" -g '*.yaml')

  local -a matches=()
  local file json count
  for file in "${candidates[@]}"; do
    json=$(bash "$LIB_DIR/yaml2json.sh" "$file" 2>/dev/null) || continue
    count=$(echo "$json" | jaq --arg id "$scenario_id" '[.scenarios[]? | select(.id == $id)] | length' 2>/dev/null || echo 0)
    if [ "$count" = "1" ]; then
      matches+=("$file")
    elif [ "$count" != "0" ]; then
      echo "ERROR: duplicate id '$scenario_id' inside file: $file"
      exit 1
    fi
  done

  if [ "${#matches[@]}" -eq 0 ]; then
    echo "ERROR: scenario id not found: $scenario_id"
    exit 1
  fi

  if [ "${#matches[@]}" -gt 1 ]; then
    echo "ERROR: scenario id '$scenario_id' matches multiple files:"
    local m
    for m in "${matches[@]}"; do
      echo "  - $m"
    done
    exit 1
  fi

  SCENARIO_FILE="${matches[0]}"
}

if [ -f "$SCENARIO_PATH" ]; then
  SCENARIO_FILE="$SCENARIO_PATH"
elif [ -d "$SCENARIO_PATH" ]; then
  if [ -z "$SCENARIO_ID" ]; then
    echo "ERROR: --id is required when scenario path is a directory"
    exit 1
  fi
  resolve_scenario_file_from_dir "$SCENARIO_PATH" "$SCENARIO_ID"
else
  echo "ERROR: unsupported scenario path: $SCENARIO_PATH"
  exit 1
fi

json=$(bash "$LIB_DIR/yaml2json.sh" "$SCENARIO_FILE")

if [ -n "$SCENARIO_ID" ]; then
  scenario=$(echo "$json" | jaq -c --arg id "$SCENARIO_ID" '.scenarios[]? | select(.id == $id)' | head -1)
else
  scenario=$(echo "$json" | jaq -c '.scenarios[0] // empty')
fi

if [ -z "$scenario" ]; then
  if [ -n "$SCENARIO_ID" ]; then
    echo "ERROR: scenario id not found: $SCENARIO_ID"
  else
    echo "ERROR: no scenarios in file"
  fi
  exit 1
fi

id=$(echo "$scenario" | jaq -r '.id // "(no-id)"')
input_file_rel=$(echo "$scenario" | jaq -r '.input_file // ""')
input_inline=$(echo "$scenario" | jaq -r '.input // ""')

# Build the --query value from actions:
#   - consecutive leading type actions → text
#   - press: Enter → appends \n (literal newline)
#   - press: Tab   → appends \t (literal tab)
#   - wait_stable / other non-type actions → stop consuming
# A plain .query/.filter field overrides action-derived text (no startup keys then).
explicit_query=$(echo "$scenario" | jaq -r '.query // .filter // ""')
if [ -n "$explicit_query" ]; then
  query="$explicit_query"
else
  query=""
  actions_json=$(echo "$scenario" | jaq -c '[.actions[]? | if type == "object" then . else {type: .} end]' 2>/dev/null || echo "[]")
  action_count=$(echo "$actions_json" | jaq 'length')
  ai=0
  while [ "$ai" -lt "$action_count" ]; do
    akey=$(echo "$actions_json" | jaq -r ".[$ai] | keys[0]" 2>/dev/null)
    aval=$(echo "$actions_json" | jaq -r ".[$ai] | .[keys[0]]" 2>/dev/null)
    case "$akey" in
      type)       query="${query}${aval}" ;;
      press)
        case "$aval" in
          Enter) query="${query}"$'\n' ;;
          Tab)   query="${query}"$'\t' ;;
          Esc)   break ;;  # Esc changes TUI state non-trivially; stop here
          *)     break ;;
        esac
        ;;
      wait_stable|wait) ;;  # skip timing-only actions
      *) break ;;
    esac
    ai=$((ai+1))
  done
fi

temp_input=""
cleanup() {
  if [ -n "$temp_input" ] && [ -f "$temp_input" ]; then
    rm -f "$temp_input"
  fi
}
trap cleanup EXIT INT TERM

if [ -n "$input_file_rel" ]; then
  if [[ "$input_file_rel" = /* ]]; then
    input_path="$input_file_rel"
  else
    input_path="$REPO_ROOT/$input_file_rel"
  fi
else
  if [ -z "$input_inline" ]; then
    echo "ERROR: scenario has neither input_file nor input"
    exit 1
  fi
  temp_input=$(mktemp)
  printf '%s\n' "$input_inline" > "$temp_input"
  input_path="$temp_input"
fi

if [ ! -f "$input_path" ]; then
  echo "ERROR: input file not found: $input_path"
  exit 1
fi

binary="$REPO_ROOT/target/debug/jqpp"
cmd=("$binary" "$input_path")
if $USE_QUERY && [ -n "$query" ]; then
  cmd+=(--query "$query")
fi

echo "Scenario: $id"
echo "Scenario File: $SCENARIO_FILE"
echo "Input: $input_path"
if $USE_QUERY && [ -n "$query" ]; then
  echo "Query: $query"
fi

if $PRINT_ONLY; then
  printf 'Command:'
  printf ' %q' "${cmd[@]}"
  echo
  exit 0
fi

cargo build --manifest-path "$REPO_ROOT/Cargo.toml"
exec "${cmd[@]}"
