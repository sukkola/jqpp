#!/usr/bin/env bash
# run-scenarios.sh — Standalone runner for jqpp test scenario YAML files.
#
# Usage:
#   run-scenarios.sh [options] <file.yaml | directory>
#
# Modes:
#   (default)           Assert all scenarios; exit 0 on pass, 1 on failure.
#   --update-baselines  Overwrite baseline screenshot files instead of comparing.
#   --capture-actuals   Run TUI scenarios and print the actual query/suggestion
#                       values for any assertion with a null (~) expected value.
#                       Prints a "Suggested assertions:" block you can paste back
#                       into the YAML. Does NOT fail on null assertions.
#   --list-scenarios    Do not execute; print scenarios matched by filters.
#
# Does NOT depend on yq.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_DIR="$SCRIPT_DIR/lib"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BASELINES_DIR="$REPO_ROOT/test-scenarios/baselines"

source "$LIB_DIR/screenshot.sh"

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

UPDATE_BASELINES=false
CAPTURE_ACTUALS=false
LIST_SCENARIOS=false
SCENARIO_FILTER=""
SCENARIO_ID_EXACT=""
QUERY_PREFIX_FILTER=""
QUERY_FUNCTION_FILTER=""
CREATED_BEFORE_FILTER=""
CREATED_WITH_VERSION_FILTER=""
TARGET=""
LIST_MATCH_COUNT=0

usage() {
  echo "Usage: run-scenarios.sh [options] <file.yaml | directory>"
  echo ""
  echo "Options:"
  echo "  --update-baselines      Overwrite existing baseline files instead of comparing"
  echo "  --capture-actuals       Print actual query/suggestion values for null (~) assertions"
  echo "                          (use this to fill in placeholders in scenario files)"
  echo "  --list-scenarios        Print matched scenarios without executing them"
  echo "  --id <id>               Run only the scenario with this exact id"
  echo "  --scenario <id>         Run only the scenario with this id (can be a substring)"
  echo "  --query-starts-with <s> Run only scenarios whose query starts with this string"
  echo "  --query-function <fn>   Run only scenarios whose query uses this function"
  echo "  --created-before <date> Include only files with meta.created < date (YYYY-MM-DD)"
  echo "  --created-with-version <v> Include only files with meta.jqpp_version containing v"
  echo ""
  echo "Exits 0 if all non-regression scenarios pass, 1 otherwise."
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --update-baselines) UPDATE_BASELINES=true; shift ;;
    --capture-actuals)  CAPTURE_ACTUALS=true; shift ;;
    --list-scenarios)   LIST_SCENARIOS=true; shift ;;
    --id)               SCENARIO_ID_EXACT="$2"; shift 2 ;;
    --scenario)         SCENARIO_FILTER="$2"; shift 2 ;;
    --query-starts-with) QUERY_PREFIX_FILTER="$2"; shift 2 ;;
    --query-function)    QUERY_FUNCTION_FILTER="$2"; shift 2 ;;
    --created-before)    CREATED_BEFORE_FILTER="$2"; shift 2 ;;
    --created-with-version) CREATED_WITH_VERSION_FILTER="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    -*) echo "ERROR: unknown option: $1"; usage; exit 1 ;;
    *) TARGET="$1"; shift ;;
  esac
done

if [ -z "$TARGET" ]; then
  echo "ERROR: no target file or directory specified"
  usage
  exit 1
fi

matches_query_filters() {
  local query="$1"

  if [ -n "$QUERY_PREFIX_FILTER" ]; then
    local trimmed
    trimmed=$(printf '%s' "$query" | sed 's/^[[:space:]]*//')
    case "$trimmed" in
      "$QUERY_PREFIX_FILTER"*) ;;
      *) return 1 ;;
    esac
  fi

  if [ -n "$QUERY_FUNCTION_FILTER" ]; then
    local compact needle
    compact=$(printf '%s' "$query" | tr -d '[:space:]')
    needle="${QUERY_FUNCTION_FILTER}("
    case "$compact" in
      *"$needle"*) ;;
      *) return 1 ;;
    esac
  fi

  return 0
}

scenario_query_for_eval() {
  local scenario_json="$1"
  echo "$scenario_json" | jaq -r '.query // .filter // ""'
}

scenario_query_for_tui() {
  local scenario_json="$1"
  echo "$scenario_json" | jaq -r '.query // ([.actions[]? | .type? // empty] | join("")) // ""'
}

print_listed_scenario() {
  local yaml_file="$1"
  local scenario_type="$2"
  local id="$3"
  local query="$4"
  LIST_MATCH_COUNT=$((LIST_MATCH_COUNT+1))
  if [ -n "$query" ]; then
    echo "LIST: ${yaml_file} [${scenario_type}] ${id} :: ${query}"
  else
    echo "LIST: ${yaml_file} [${scenario_type}] ${id}"
  fi
}

matches_meta_filters() {
  local meta_created="$1"
  local meta_version="$2"

  if [ -n "$CREATED_BEFORE_FILTER" ]; then
    case "$meta_created" in
      ????-??-??)
        if ! [[ "$meta_created" < "$CREATED_BEFORE_FILTER" ]]; then
          return 1
        fi
        ;;
      *)
        return 1
        ;;
    esac
  fi

  if [ -n "$CREATED_WITH_VERSION_FILTER" ] && ! echo "$meta_version" | grep -qF "$CREATED_WITH_VERSION_FILTER"; then
    return 1
  fi

  return 0
}

# ---------------------------------------------------------------------------
# Dependency checks
# ---------------------------------------------------------------------------

if ! command -v jaq >/dev/null 2>&1; then
  echo "ERROR: jaq not found. Run: mise install"
  exit 1
fi

if ! command -v agent-tui >/dev/null 2>&1; then
  echo "ERROR: agent-tui not found. Run: mise install"
  exit 1
fi

JQ_AVAILABLE=false
if command -v jq >/dev/null 2>&1; then
  JQ_AVAILABLE=true
fi

# ---------------------------------------------------------------------------
# Version capture
# ---------------------------------------------------------------------------

JQPP_VERSION="unknown"
JQPP_GIT_SHA="unknown"

if [ -f "$REPO_ROOT/target/debug/jqpp" ]; then
  VERSION_LINE=$("$REPO_ROOT/target/debug/jqpp" --version 2>/dev/null | head -1 || echo "")
  # Extract semver (first word after binary name)
  JQPP_VERSION=$(echo "$VERSION_LINE" | awk '{print $2}')
  # Extract build tag from parens: "jqpp 0.1.0 (8020921)" → "8020921"
  JQPP_GIT_SHA=$(echo "$VERSION_LINE" | sed -n 's/.*(\(.*\))/\1/p')
  [ -z "$JQPP_GIT_SHA" ] && JQPP_GIT_SHA="unknown"
elif command -v cargo >/dev/null 2>&1; then
  JQPP_VERSION=$(cargo metadata --manifest-path "$REPO_ROOT/Cargo.toml" \
    --no-deps --format-version 1 2>/dev/null \
    | jaq -r '.packages[0].version' 2>/dev/null || echo "unknown")
  JQPP_GIT_SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
fi

echo "jqpp test runner"
echo "  jqpp: $JQPP_VERSION ($JQPP_GIT_SHA)"
echo "  jaq:  $(jaq --version 2>/dev/null || echo unknown)"
echo "  jq:   $($JQ_AVAILABLE && jq --version 2>/dev/null || echo 'not available')"
echo ""

# ---------------------------------------------------------------------------
# Collect scenario files
# ---------------------------------------------------------------------------

collect_files() {
  local target="$1"
  if [ -f "$target" ]; then
    echo "$target"
  elif [ -d "$target" ]; then
    find "$target" -name '*.yaml' | sort
  else
    echo "ERROR: target not found: $target" >&2
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# Summary counters
# ---------------------------------------------------------------------------

TOTAL_PASS=0
TOTAL_FAIL=0
TOTAL_EXPECTED_FAIL=0
TOTAL_SKIP=0

# ---------------------------------------------------------------------------
# Run a single evaluation scenario (has 'filter' key, no agent-tui)
# ---------------------------------------------------------------------------

run_eval_scenario() {
  local scenario_file="$1"
  local id="$2"
  local description="$3"
  local input="$4"
  local filter="$5"
  local expected="$6"
  local status="${7:-active}"
  local jq_diverges="${8:-false}"
  local scenario_ref="${scenario_file} --id ${id}"

  if [ "$status" = "skip" ]; then
    echo "SKIP: $scenario_ref"
    TOTAL_SKIP=$((TOTAL_SKIP+1))
    return
  fi

  # Normalize trailing whitespace in expected output
  expected=$(echo "$expected" | sed 's/[[:space:]]*$//')

  local actual
  actual=$(echo "$input" | jaq "$filter" 2>/dev/null) || actual=""
  actual=$(echo "$actual" | sed 's/[[:space:]]*$//')

  local passed=false
  [ "$actual" = "$expected" ] && passed=true

  # jq secondary validation
  if $JQ_AVAILABLE; then
    local jq_actual
    jq_actual=$(echo "$input" | jq "$filter" 2>/dev/null) || jq_actual=""
    jq_actual=$(echo "$jq_actual" | sed 's/[[:space:]]*$//')
    if [ "$actual" != "$jq_actual" ]; then
      if [ "$jq_diverges" = "true" ]; then
         echo "  INFO: jq produces different output for $scenario_ref (jq_diverges is set)"
        echo "    jaq: $actual"
        echo "    jq:  $jq_actual"
      else
         echo "  WARN: jaq/jq diverge on $scenario_ref — consider setting jq_diverges: true"
        echo "    jaq: $actual"
        echo "    jq:  $jq_actual"
      fi
    fi
  fi

  if $passed; then
    if [ "$status" = "regression" ]; then
      echo "UNEXPECTED PASS: $scenario_ref  (was marked regression)"
      TOTAL_FAIL=$((TOTAL_FAIL+1))
    elif [ "$jq_diverges" = "true" ]; then
      echo "PASS (jaq): $scenario_ref"
      TOTAL_PASS=$((TOTAL_PASS+1))
    else
      echo "PASS: $scenario_ref"
      TOTAL_PASS=$((TOTAL_PASS+1))
    fi
  else
    if [ "$status" = "regression" ]; then
      echo "EXPECTED FAIL: $scenario_ref"
      TOTAL_EXPECTED_FAIL=$((TOTAL_EXPECTED_FAIL+1))
    else
      echo "FAIL: $scenario_ref"
      echo "  expected: $(echo "$expected" | head -3)"
      echo "  actual:   $(echo "$actual"   | head -3)"
      echo "  reproduce: mise run tui-launch-scenario \"$scenario_file\" --id \"$id\""
      TOTAL_FAIL=$((TOTAL_FAIL+1))
    fi
  fi
}

# ---------------------------------------------------------------------------
# Run a single TUI scenario (has 'actions' key, requires agent-tui)
# ---------------------------------------------------------------------------

run_tui_scenario() {
  local scenario_file="$1"
  local id="$2"
  local description="$3"
  local input_file="$4"
  local actions_json="$5"
  local assertions_json="$6"
  local asserts_with_json="$7"
  local status="${8:-active}"
  local scenario_ref="${scenario_file} --id ${id}"

  if [ "$status" = "skip" ]; then
    echo "SKIP: $scenario_ref"
    TOTAL_SKIP=$((TOTAL_SKIP+1))
    return
  fi

  # Determine which lib functions to call
  local all_fns="tui_suggestion_box_visible tui_extract_suggestions tui_read_query"
  local fns_to_run
  if [ "$asserts_with_json" = "null" ] || [ -z "$asserts_with_json" ]; then
    echo "  WARN: asserts_with not set for $id; running all lib functions"
    fns_to_run="$all_fns"
  else
    fns_to_run=$(echo "$asserts_with_json" | jaq -r '.[]' 2>/dev/null | tr '\n' ' ')
  fi

  # Resolve input_file relative to repo root
  local abs_input="$REPO_ROOT/$input_file"
  if [ ! -f "$abs_input" ]; then
    echo "FAIL: $scenario_ref — input_file not found: $input_file"
    TOTAL_FAIL=$((TOTAL_FAIL+1))
    return
  fi

  # Launch jqpp session.
  # Always preload leading consecutive type actions as --query so they bypass
  # the TUI's keystroke autocomplete, which can mangle complex strings with
  # brackets, quotes, etc. Only replay actions after the leading type block.
  local action_count preload_query first_action_idx
  action_count=$(echo "$actions_json" | jaq 'length' 2>/dev/null || echo 0)
  preload_query=""
  first_action_idx=0

  local pi=0
  while [ "$pi" -lt "$action_count" ]; do
    local pak
    pak=$(echo "$actions_json" | jaq -r ".[$pi] | keys[0]" 2>/dev/null)
    if [ "$pak" = "type" ]; then
      local pav
      pav=$(echo "$actions_json" | jaq -r ".[$pi].type" 2>/dev/null)
      preload_query="${preload_query}${pav}"
      first_action_idx=$((pi+1))
      pi=$((pi+1))
    else
      break
    fi
  done

  local -a launch_cmd
  launch_cmd=(agent-tui run --cols 220 --format json --cwd "$REPO_ROOT" "$REPO_ROOT/target/debug/jqpp" "$abs_input")
  if [ -n "$preload_query" ]; then
    launch_cmd+=(--query "$preload_query")
  fi

  local session
  session=$("${launch_cmd[@]}" 2>/dev/null | jaq -r '.session_id' 2>/dev/null) || {
    echo "FAIL: $scenario_ref — could not launch agent-tui session"
    TOTAL_FAIL=$((TOTAL_FAIL+1))
    return
  }

  # Cleanup on exit (even on failure)
  local cleanup_done=false
  cleanup_session() {
    if ! $cleanup_done; then
      cleanup_done=true
      agent-tui -s "$session" kill >/dev/null 2>&1 || true
    fi
  }
  trap cleanup_session INT TERM

  # Wait for Query bar
  if ! agent-tui -s "$session" wait "Query" --assert -t 10000 >/dev/null 2>&1; then
    echo "FAIL: $scenario_ref — ERROR: timeout waiting for TUI"
    TOTAL_FAIL=$((TOTAL_FAIL+1))
    cleanup_session
    return
  fi

  # Execute remaining actions (after the preloaded type block).
  local i=$first_action_idx
  while [ "$i" -lt "$action_count" ]; do
    local action_type action_value
    action_type=$(echo "$actions_json" | jaq -r ".[$i] | keys[0]" 2>/dev/null)
    action_value=$(echo "$actions_json" | jaq -r ".[$i] | .${action_type}" 2>/dev/null)

    case "$action_type" in
      type)
        agent-tui -s "$session" type "$action_value" >/dev/null 2>&1
        ;;
      press)
        agent-tui -s "$session" press "$action_value" >/dev/null 2>&1
        ;;
      wait_stable)
        agent-tui -s "$session" wait --stable >/dev/null 2>&1
        ;;
    esac
    i=$((i+1))
  done

  # Wait for screen to stabilize after actions
  agent-tui -s "$session" wait --stable >/dev/null 2>&1 || true

  # Capture screenshot
  local screen
  screen=$(agent-tui -s "$session" screenshot --strip-ansi 2>/dev/null)

  # Write raw baseline
  mkdir -p "$BASELINES_DIR"
  if $UPDATE_BASELINES || [ ! -f "$BASELINES_DIR/${id}-raw.txt" ]; then
    echo "$screen" > "$BASELINES_DIR/${id}-raw.txt"
  fi

  # Run lib functions and write function-specific baselines
  for fn in $fns_to_run; do
    local fn_output
    fn_output=$($fn "$screen" 2>/dev/null; echo "EXIT:$?") || true
    local fn_exit
    fn_exit=$(echo "$fn_output" | tail -1 | sed 's/EXIT://')
    fn_output=$(printf '%s\n' "$fn_output" | sed '$d')

    if $UPDATE_BASELINES || [ ! -f "$BASELINES_DIR/${id}-${fn}.txt" ]; then
      echo "$fn_output" > "$BASELINES_DIR/${id}-${fn}.txt"
    fi
  done

  # Capture actuals once (used for both assertions and --capture-actuals output)
  local actual_query actual_suggestions
  actual_query=$(tui_read_query "$screen")
  actual_suggestions=$(tui_extract_suggestions "$screen")

  # --capture-actuals: print actual values for any null assertion, then skip asserting
  if $CAPTURE_ACTUALS; then
    local has_nulls=false
    local assert_count_ca
    assert_count_ca=$(echo "$assertions_json" | jaq 'length' 2>/dev/null || echo 0)
    local jca=0
    while [ "$jca" -lt "$assert_count_ca" ]; do
      local ak av
      ak=$(echo "$assertions_json" | jaq -r ".[$jca] | keys[0]" 2>/dev/null)
      av=$(echo "$assertions_json" | jaq -r ".[$jca].${ak}" 2>/dev/null)
      if [ "$av" = "null" ]; then has_nulls=true; fi
      jca=$((jca+1))
    done

    if $has_nulls; then
      echo "CAPTURE: $id"
      echo "  Suggested assertions (paste into YAML replacing ~ values):"
      local jca2=0
      while [ "$jca2" -lt "$assert_count_ca" ]; do
        local ak2 av2
        ak2=$(echo "$assertions_json" | jaq -r ".[$jca2] | keys[0]" 2>/dev/null)
        av2=$(echo "$assertions_json" | jaq -r ".[$jca2].${ak2}" 2>/dev/null)
        if [ "$av2" = "null" ]; then
          case "$ak2" in
            query_equals|query_contains)
              echo "      - ${ak2}: \"${actual_query}\""
              ;;
            suggestions_contain)
              local sugg_yaml
              sugg_yaml=$(echo "$actual_suggestions" | awk 'NF{printf "\"" $0 "\", "}' | sed 's/, $//')
              echo "      - ${ak2}: [${sugg_yaml}]"
              ;;
          esac
        fi
        jca2=$((jca2+1))
      done
      cleanup_session
      return
    fi
  fi

  # Evaluate assertions
  local scenario_passed=true
  local fail_reason=""

  local assert_count
  assert_count=$(echo "$assertions_json" | jaq 'length' 2>/dev/null || echo 0)
  local j=0
  while [ "$j" -lt "$assert_count" ]; do
    local assert_key assert_val
    assert_key=$(echo "$assertions_json" | jaq -r ".[$j] | keys[0]" 2>/dev/null)
    assert_val=$(echo "$assertions_json" | jaq -r ".[$j].${assert_key}" 2>/dev/null)

    # Skip null placeholders (not yet filled in)
    if [ "$assert_val" = "null" ]; then
      echo "  WARN: $id — ${assert_key} has no expected value (~); skipping assertion (run --capture-actuals to fill in)"
      j=$((j+1))
      continue
    fi

    case "$assert_key" in
      suggestions_visible)
        if [ "$assert_val" = "true" ]; then
          if ! tui_suggestion_box_visible "$screen"; then
            scenario_passed=false
            fail_reason="suggestions_visible: expected box to be visible but it was not"
          fi
        else
          if tui_suggestion_box_visible "$screen"; then
            scenario_passed=false
            fail_reason="suggestions_visible: expected box to be hidden but it was visible"
          fi
        fi
        ;;
      suggestions_contain)
        local required_labels
        required_labels=$(echo "$assertions_json" | jaq -r ".[$j].suggestions_contain[]" 2>/dev/null)
        while IFS= read -r label; do
          [ -n "$label" ] || continue
          if ! echo "$actual_suggestions" | grep -qF "$label"; then
            scenario_passed=false
            fail_reason="suggestions_contain: '$label' not found"$'\n'"  actual suggestions: $(echo "$actual_suggestions" | tr '\n' '|')"$'\n'"  raw screen:"$'\n'"$(echo "$screen" | sed 's/^/    /')"
            break
          fi
        done <<< "$required_labels"
        ;;
      suggestions_not_contain)
        local forbidden_labels
        forbidden_labels=$(echo "$assertions_json" | jaq -r ".[$j].suggestions_not_contain[]" 2>/dev/null)
        while IFS= read -r label; do
          [ -n "$label" ] || continue
          if echo "$actual_suggestions" | grep -qxF "$label"; then
            scenario_passed=false
            fail_reason="suggestions_not_contain: '$label' found but should not be"$'\n'"  actual suggestions: $(echo "$actual_suggestions" | tr '\n' '|')"$'\n'"  raw screen:"$'\n'"$(echo "$screen" | sed 's/^/    /')"
            break
          fi
        done <<< "$forbidden_labels"
        ;;
      query_contains)
        if ! echo "$actual_query" | grep -qF "$assert_val"; then
          scenario_passed=false
          fail_reason="query_contains: '$assert_val' not found in query bar"$'\n'"  actual query: '$actual_query'"
        fi
        ;;
      query_equals)
        if [ "$actual_query" != "$assert_val" ]; then
          scenario_passed=false
          fail_reason="query_equals: mismatch"$'\n'"  expected: '$assert_val'"$'\n'"  actual:   '$actual_query'"
        fi
        ;;
    esac
    j=$((j+1))
  done

  cleanup_session

  if $scenario_passed; then
    if [ "$status" = "regression" ]; then
      echo "UNEXPECTED PASS: $scenario_ref  (was marked regression)"
      TOTAL_FAIL=$((TOTAL_FAIL+1))
    else
      echo "PASS: $scenario_ref"
      TOTAL_PASS=$((TOTAL_PASS+1))
    fi
  else
    if [ "$status" = "regression" ]; then
      echo "EXPECTED FAIL: $scenario_ref"
      TOTAL_EXPECTED_FAIL=$((TOTAL_EXPECTED_FAIL+1))
    else
      echo "FAIL: $scenario_ref"
      echo "  $fail_reason"
      echo "  reproduce: mise run tui-launch-scenario \"$scenario_file\" --id \"$id\""
      TOTAL_FAIL=$((TOTAL_FAIL+1))
    fi
  fi
}

# ---------------------------------------------------------------------------
# Process a single scenario file
# ---------------------------------------------------------------------------

run_file() {
  local yaml_file="$1"

  echo "--- $yaml_file ---"

  # Convert YAML → JSON
  local json
  json=$(bash "$LIB_DIR/yaml2json.sh" "$yaml_file") || {
    echo "ERROR: failed to parse $yaml_file"
    TOTAL_FAIL=$((TOTAL_FAIL+1))
    return
  }

  # Validate meta block
  local meta_version meta_sha meta_created meta_topic meta_desc
  meta_version=$(echo "$json" | jaq -r '.meta.jqpp_version // empty' 2>/dev/null)
  meta_sha=$(echo "$json"     | jaq -r '.meta.jqpp_git_sha  // empty' 2>/dev/null)
  meta_created=$(echo "$json" | jaq -r '.meta.created       // empty' 2>/dev/null)
  meta_topic=$(echo "$json"   | jaq -r '.meta.topic         // empty' 2>/dev/null)
  meta_desc=$(echo "$json"    | jaq -r '.meta.description   // empty' 2>/dev/null)

  for field in meta_version meta_sha meta_created meta_topic meta_desc; do
    val="${!field}"
    if [ -z "$val" ]; then
      field_name="${field#meta_}"
      echo "ERROR: missing meta.${field_name} in $yaml_file"
      TOTAL_FAIL=$((TOTAL_FAIL+1))
      return
    fi
  done

  # Optional metadata filters (useful with --list-scenarios)
  if ! matches_meta_filters "$meta_created" "$meta_version"; then
    return
  fi

  # Version mismatch warning — compare git SHAs since the semver in Cargo.toml
  # may not be bumped between builds. SHA is the reliable identifier.
  if [ "$meta_sha" != "$JQPP_GIT_SHA" ] && [ "$JQPP_GIT_SHA" != "unknown" ]; then
    echo "  WARN: scenario created against $meta_sha, running against $JQPP_GIT_SHA (version: $meta_version → $JQPP_VERSION)"
  fi

  echo "  topic: $meta_topic — $meta_desc"
  echo ""

  # Run evaluation scenarios (have 'filter' key)
  local eval_count
  eval_count=$(echo "$json" | jaq '[.scenarios[]? | select(.filter)] | length' 2>/dev/null || echo 0)
  if [ "$eval_count" -gt 0 ]; then
    local i=0
    while [ "$i" -lt "$eval_count" ]; do
      # Extract index into the filtered list
      local scenario
      scenario=$(echo "$json" | jaq ".scenarios | map(select(.filter)) | .[$i]" 2>/dev/null)

      local id description input filter expected status jq_diverges query
      id=$(echo "$scenario"          | jaq -r '.id')
      filter=$(echo "$scenario"      | jaq -r '.filter')
      query=$(scenario_query_for_eval "$scenario")

      # --id exact filter
      if [ -n "$SCENARIO_ID_EXACT" ] && [ "$id" != "$SCENARIO_ID_EXACT" ]; then
        i=$((i+1)); continue
      fi

      # --scenario substring filter
      if [ -n "$SCENARIO_FILTER" ] && ! echo "$id" | grep -qF "$SCENARIO_FILTER"; then
        i=$((i+1)); continue
      fi

      if ! matches_query_filters "$query"; then
        i=$((i+1)); continue
      fi

      if $LIST_SCENARIOS; then
        print_listed_scenario "$yaml_file" "eval" "$id" "$query"
        i=$((i+1)); continue
      fi

      description=$(echo "$scenario" | jaq -r '.description // ""')
      input=$(echo "$scenario"       | jaq -r '.input')
      expected=$(echo "$scenario"    | jaq -r '.expected_output')
      status=$(echo "$scenario"      | jaq -r '.status // "active"')
      jq_diverges=$(echo "$scenario" | jaq -r '.jq_diverges // "false"')

      run_eval_scenario "$yaml_file" "$id" "$description" "$input" "$filter" "$expected" "$status" "$jq_diverges"
      i=$((i+1))
    done
  fi

  # Run TUI scenarios (have 'actions' key)
  local tui_count
  tui_count=$(echo "$json" | jaq '[.scenarios[]? | select(.actions)] | length' 2>/dev/null || echo 0)
  if [ "$tui_count" -gt 0 ]; then
    local i=0
    while [ "$i" -lt "$tui_count" ]; do
      local scenario
      scenario=$(echo "$json" | jaq ".scenarios | map(select(.actions)) | .[$i]" 2>/dev/null)

      local id description input_file actions assertions asserts_with status query
      id=$(echo "$scenario"            | jaq -r '.id')
      query=$(scenario_query_for_tui "$scenario")

      # --id exact filter
      if [ -n "$SCENARIO_ID_EXACT" ] && [ "$id" != "$SCENARIO_ID_EXACT" ]; then
        i=$((i+1)); continue
      fi

      # --scenario substring filter
      if [ -n "$SCENARIO_FILTER" ] && ! echo "$id" | grep -qF "$SCENARIO_FILTER"; then
        i=$((i+1)); continue
      fi

      if [ -n "$QUERY_PREFIX_FILTER" ] || [ -n "$QUERY_FUNCTION_FILTER" ]; then
        if [ -z "$query" ] || ! matches_query_filters "$query"; then
          i=$((i+1)); continue
        fi
      fi

      if $LIST_SCENARIOS; then
        print_listed_scenario "$yaml_file" "tui" "$id" "$query"
        i=$((i+1)); continue
      fi

      description=$(echo "$scenario"  | jaq -r '.description // ""')
      input_file=$(echo "$scenario"   | jaq -r '.input_file // "demo/demo.json"')
      actions=$(echo "$scenario"      | jaq '.actions')
      assertions=$(echo "$scenario"   | jaq '.assertions // []')
      asserts_with=$(echo "$scenario" | jaq -r '.asserts_with // "null"')
      status=$(echo "$scenario"       | jaq -r '.status // "active"')

      run_tui_scenario "$yaml_file" "$id" "$description" "$input_file" "$actions" "$assertions" "$asserts_with" "$status"
      i=$((i+1))
    done
  fi

  echo ""
}

# ---------------------------------------------------------------------------
# Main: iterate over collected files
# ---------------------------------------------------------------------------

while IFS= read -r yaml_file; do
  run_file "$yaml_file"
done < <(collect_files "$TARGET")

if $LIST_SCENARIOS; then
  echo "Matched scenarios: $LIST_MATCH_COUNT"
  [ "$LIST_MATCH_COUNT" -gt 0 ] || exit 1
  exit 0
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

echo "Results: $TOTAL_PASS passed, $TOTAL_FAIL failed, $TOTAL_EXPECTED_FAIL expected-fail, $TOTAL_SKIP skipped"

[ "$TOTAL_FAIL" -eq 0 ] || exit 1
