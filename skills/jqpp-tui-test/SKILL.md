---
name: jqpp-tui-test
description: >
  Run scripted TUI tests against jqpp. Launches the app with agent-tui, drives keyboard sequences
  through the query bar, captures and validates suggestion box content and position, and compares
  final query output against expected values. Use this for regression testing intellisense behaviour,
  validating Tab/Enter/Esc acceptance flows, and generating reproducible test evidence.
user-invocable: true
allowed-tools: Bash, Read, Write, Glob, Grep
---

# jqpp TUI Test Skill

Automate and validate intellisense behaviour in the jqpp TUI using agent-tui.

## App launch

Always launch from the repo root with the demo JSON file (or a custom fixture):

```bash
agent-tui run --format json --cwd /Users/sampo/Sources/jqt \
  target/debug/jqpp demo/demo.json
# Capture session_id from JSON output: .session_id
```

Wait for the app to finish loading before interacting:

```bash
agent-tui wait "Query" --assert -t 10000
```

The app is ready when the Query bar border is visible. Two distinct waiting conditions:
- `agent-tui wait "Query" --assert` — UI frame is painted
- `agent-tui wait --stable --assert` — screen has stopped changing

## Key bindings reference

| Key string | Meaning |
|---|---|
| `Tab` | Accept highlighted suggestion / advance completion |
| `BackTab` | Previous suggestion |
| `Enter` | Evaluate query |
| `Escape` | Close suggestion box / finalize contains builder |
| `ctrl+y` | Copy query+input+output to clipboard (text snapshot) |
| `ctrl+c` | Quit |

Press a single key:
```bash
agent-tui press Tab
agent-tui press Enter
agent-tui press Escape
agent-tui press BackTab
```

Type literal characters one by one (triggers intellisense):
```bash
agent-tui type "."
agent-tui type "orders"
agent-tui press "|"      # pipe character
```

## Detecting the suggestion box

Use the shared screenshot library (`scripts/tui/lib/screenshot.sh`):

```bash
source scripts/tui/lib/screenshot.sh
SCREEN=$(agent-tui -s "$SESSION" screenshot --strip-ansi)

# Check if suggestion box is visible (returns 0/1)
tui_suggestion_box_visible "$SCREEN"

# Extract suggestion labels (one per line)
tui_extract_suggestions "$SCREEN"

# Read current query bar text
tui_read_query "$SCREEN"
```

Capture once and pass `"$SCREEN"` to all functions — no extra agent-tui calls needed.

### Verify a specific suggestion is visible

```bash
source scripts/tui/lib/screenshot.sh
SCREEN=$(agent-tui -s "$SESSION" screenshot --strip-ansi)
SUGGESTIONS=$(tui_extract_suggestions "$SCREEN")

echo "$SUGGESTIONS" | grep -qF "orders" && echo "PASS: 'orders' found" || echo "FAIL: 'orders' not found"
```

## Standard test sequence skeleton

```bash
#!/usr/bin/env bash
set -euo pipefail

BINARY="target/debug/jqpp"
INPUT_FILE="demo/demo.json"
CWD="/Users/sampo/Sources/jqt"

# 1. Launch
SESSION=$(agent-tui run --format json --cwd "$CWD" "$BINARY" "$INPUT_FILE" \
  | jq -r '.session_id')
echo "Session: $SESSION"

# 2. Wait for ready
agent-tui -s "$SESSION" wait "Query" --assert -t 10000

# 3. Type query prefix to trigger suggestions
agent-tui -s "$SESSION" type "."
agent-tui -s "$SESSION" wait --stable

# 4. Capture screen and extract suggestions
SCREEN=$(agent-tui -s "$SESSION" screenshot --strip-ansi)
SUGGESTIONS=$(echo "$SCREEN" | grep -oP '(?<=│)[^│]+(?=│)' \
  | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' \
  | grep -v '^$' | grep -v '^─')

echo "=== Suggestions ==="
echo "$SUGGESTIONS"

# 5. Assert expected suggestion is present
echo "$SUGGESTIONS" | grep -qF "orders" \
  && echo "PASS: 'orders' in suggestions" \
  || { echo "FAIL: 'orders' not found"; agent-tui -s "$SESSION" kill; exit 1; }

# 6. Accept suggestion with Tab
agent-tui -s "$SESSION" press Tab
agent-tui -s "$SESSION" wait --stable

# 7. Capture query bar state
QUERY_ROW=$(agent-tui -s "$SESSION" screenshot --strip-ansi \
  | grep -A1 "Query" | tail -1 | sed 's/│//g' | sed 's/^[[:space:]]*//')
echo "Query after Tab: $QUERY_ROW"

# 8. Evaluate with Enter and capture output
agent-tui -s "$SESSION" press Enter
agent-tui -s "$SESSION" wait --stable

SCREEN_AFTER=$(agent-tui -s "$SESSION" screenshot --strip-ansi)

# 9. Cleanup
agent-tui -s "$SESSION" kill

echo "=== Final screen ==="
echo "$SCREEN_AFTER"
```

## Validating query output with jq

After a test produces a query, validate it independently using jq directly:

```bash
QUERY=".orders[0].customer.customer_name"
INPUT_FILE="demo/demo.json"
EXPECTED='"Alice Korhonen"'

ACTUAL=$(jq "$QUERY" "$INPUT_FILE")
[ "$ACTUAL" = "$EXPECTED" ] \
  && echo "PASS: query output matches" \
  || echo "FAIL: got '$ACTUAL', expected '$EXPECTED'"
```

## Scenario file format

Store scenarios as YAML files in `test-scenarios/` (evaluation) or `test-scenarios/intellisense/` (TUI).
Run them with the standalone runner:

```bash
scripts/tui/run-scenarios.sh test-scenarios/select.yaml
scripts/tui/run-scenarios.sh test-scenarios/          # all files recursively
```

See `test-scenarios/README.md` for the full YAML format reference.

## Intellisense-specific test patterns

### Test: suggestion box appears after trigger character

```bash
agent-tui type "."
agent-tui wait --stable
SCREEN=$(agent-tui screenshot --strip-ansi)
echo "$SCREEN" | grep -q "│" && echo "PASS: suggestion box visible" || echo "FAIL"
```

### Test: Tab accepts top suggestion and updates query

```bash
agent-tui type "."
agent-tui wait --stable
BEFORE_QUERY=$(agent-tui screenshot --strip-ansi | grep -A1 "Query" | tail -1)
agent-tui press Tab
agent-tui wait --stable
AFTER_QUERY=$(agent-tui screenshot --strip-ansi | grep -A1 "Query" | tail -1)
[ "$BEFORE_QUERY" != "$AFTER_QUERY" ] && echo "PASS: query changed" || echo "FAIL"
```

### Test: Esc closes suggestion box

```bash
agent-tui type "."
agent-tui wait --stable
agent-tui press Escape
agent-tui wait --stable
SCREEN=$(agent-tui screenshot --strip-ansi)
# After Esc, no suggestion border lines should appear inside Input area
echo "$SCREEN" | grep -c "└.*│.*│" | grep -q "^0$" && echo "PASS: box closed" || echo "FAIL"
```

### Test: contains() wizard produces valid query

```bash
agent-tui type ".orders|contains("
agent-tui wait --stable
# Suggestions should include field names with [{  prefix
SUGG=$(agent-tui screenshot --strip-ansi | grep -oP '(?<=│)[^│]+(?=│)' | grep '\[\{')
echo "contains suggestions: $SUGG"
agent-tui press Tab        # accept first field key
agent-tui wait --stable
agent-tui press Tab        # accept sub-key
agent-tui wait --stable
agent-tui press Tab        # accept value
agent-tui wait --stable
agent-tui press Escape     # close/finalize builder
agent-tui wait --stable
QUERY=$(agent-tui screenshot --strip-ansi | grep -A1 "Query" | tail -1 | tr -d '│ ')
echo "Final query: $QUERY"
# Validate the resulting query works with jq
jq "$QUERY" demo/demo.json && echo "PASS: query valid" || echo "FAIL: query invalid"
```

## Extracting ctrl+y snapshot

The TUI copies the current query+input+output as text when ctrl+y is pressed. 
Capture via clipboard (macOS):

```bash
agent-tui press "ctrl+y"
sleep 0.3
pbpaste  # or xclip/xsel on Linux
```

This gives a structured text snapshot that can be stored as a test expectation.

## Running multiple test cases

```bash
#!/usr/bin/env bash
PASS=0; FAIL=0
for f in tests/tui/cases/*.sh; do
  bash "$f" && PASS=$((PASS+1)) || FAIL=$((FAIL+1))
done
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
```
