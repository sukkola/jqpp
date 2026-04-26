#!/usr/bin/env bash
# screenshot_test.sh — Unit tests for screenshot.sh using heredoc screen fixtures.
# Run directly: bash scripts/tui/lib/screenshot_test.sh
# No agent-tui required.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/screenshot.sh"

PASS=0
FAIL=0

assert_true() {
  local desc="$1"; shift
  if "$@"; then
    echo "PASS: $desc"
    PASS=$((PASS+1))
  else
    echo "FAIL: $desc"
    FAIL=$((FAIL+1))
  fi
}

assert_eq() {
  local desc="$1"
  local expected="$2"
  local actual="$3"
  if [ "$actual" = "$expected" ]; then
    echo "PASS: $desc"
    PASS=$((PASS+1))
  else
    echo "FAIL: $desc"
    echo "  expected: $(echo "$expected" | head -5)"
    echo "  actual:   $(echo "$actual"   | head -5)"
    FAIL=$((FAIL+1))
  fi
}

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

# A screen with a visible suggestion box containing three suggestions (with spaces).
read -r -d '' SCREEN_WITH_BOX <<'FIXTURE' || true
┌─────────────────── Query ───────────────────┐
│ .orders                                      │
├──────────────────────────────────────────────┤
│ orders                                       │
│ metadata                                     │
│ user                                         │
└──────────────────────────────────────────────┘
┌─────────────────── Output ──────────────────┐
│ [...]                                        │
└──────────────────────────────────────────────┘
FIXTURE

# Legacy box where suggestion text starts right after │ with no leading space.
read -r -d '' SCREEN_WITH_BOX_NOSPACE <<'FIXTURE' || true
┌─────────────────── Query ───────────────────┐
│ .orders                                      │
├──────────────────────────────────────────────┤
│orders                                        │
│metadata                                      │
│user                                          │
└──────────────────────────────────────────────┘
┌─────────────────── Output ──────────────────┐
│ [...]                                        │
└──────────────────────────────────────────────┘
FIXTURE

# A screen with NO suggestion box (just the two main panes).
read -r -d '' SCREEN_NO_BOX <<'FIXTURE' || true
┌─────────────────── Query ───────────────────┐
│ .orders                                      │
└──────────────────────────────────────────────┘
┌─────────────────── Output ──────────────────┐
│ [...]                                        │
└──────────────────────────────────────────────┘
FIXTURE

# A screen with a separator row among suggestions.
read -r -d '' SCREEN_WITH_SEPARATOR <<'FIXTURE' || true
┌─────────────────── Query ───────────────────┐
│ .                                            │
├──────────────────────────────────────────────┤
│ orders                                       │
│ ─────────────────────────────────────────── │
│ metadata                                     │
└──────────────────────────────────────────────┘
FIXTURE

# A screen where the query bar has content.
read -r -d '' SCREEN_QUERY_TEXT <<'FIXTURE' || true
┌─────────────────── Query ───────────────────┐
│ .orders[]                                    │
└──────────────────────────────────────────────┘
FIXTURE

# A screen where the query bar is empty.
read -r -d '' SCREEN_QUERY_EMPTY <<'FIXTURE' || true
┌─────────────────── Query ───────────────────┐
│                                              │
└──────────────────────────────────────────────┘
FIXTURE

# A screen with overlay-style suggestion box (no ├ separator row).
read -r -d '' SCREEN_OVERLAY_BOX <<'FIXTURE' || true
┌ Query ───────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│select(                                                                                                               │
└──────│.metadata    │─────────────────────────────────────────────────────────────────────────────────────────────────┘
 ┌ Inpu│.orders      │────────────────────────────────────┐▲ ┌ Output ────────────────────────────────────────────────┐
 │{    │.products    │                                    │█ │                                                        │
 │  "st│.store_id    │-001",                              │█ │                                                        │
 │  "st│.store_name  │dic Widgets",                       │█ │                                                        │
 │  "st│.store_region│U-NORTH",                           │█ │                                                        │
 │  "me│.            │                                    │█ │                                                        │
 │    "│has(         │2024-01-15T10:00:00Z",              │█ │                                                        │
 │    "│type         │                                    │█ │                                                        │
 │  }, │length       │                                    │█ │                                                        │
 │  "or│keys         │                                    │█ │                                                        │
 │    {└─────────────┘                                    │║ │                                                        │
FIXTURE

# A screen with a standalone popup suggestion box below the query bar
# (suggestions in a separate ┌──┐ box, not overlaid on the query border).
read -r -d '' SCREEN_POPUP_BOX <<'FIXTURE' || true
┌ Query ───────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│.orders | contains([{customer: {customer_email: "alice@example.com"} ,                                                │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
┌──────────────┐
│customer      │
│items         │
│totals        │
└──────────────┘
 ┌ Input ─────────────────────────────────────────────────┐▲ ┌ Output ────────────────────────────────────────────────┐
 │{                                                       │█ │false                                                   │
FIXTURE

# Popup rows without a ┌──┐ header — bare │word│ lines after query close.
read -r -d '' SCREEN_POPUP_BARE <<'FIXTURE' || true
┌ Query ───────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│.orders | contains([{customer: {customer_email: "alice@example.com"} ,                                                │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
│customer      │
│items         │
│totals        │
└──────────────┘
 ┌ Input ─────────────────────────────────────────────────┐▲ ┌ Output ────────────────────────────────────────────────┐
 │{                                                       │█ │false                                                   │
FIXTURE

# ---------------------------------------------------------------------------
# tui_suggestion_box_visible
# ---------------------------------------------------------------------------

assert_true "suggestion box present → returns 0" \
  tui_suggestion_box_visible "$SCREEN_WITH_BOX"

assert_true "no suggestion box → returns 1" \
  bash -c 'source '"$SCRIPT_DIR"'/screenshot.sh; ! tui_suggestion_box_visible "$1"' _ "$SCREEN_NO_BOX"

# ---------------------------------------------------------------------------
# tui_extract_suggestions
# ---------------------------------------------------------------------------

SUGGESTIONS=$(tui_extract_suggestions "$SCREEN_WITH_BOX")
assert_eq "extract three suggestions (with spaces)" "$(printf 'orders\nmetadata\nuser')" "$SUGGESTIONS"

SUGGESTIONS_NOSPACE=$(tui_extract_suggestions "$SCREEN_WITH_BOX_NOSPACE")
assert_eq "extract three suggestions (no space after pipe)" "$(printf 'orders\nmetadata\nuser')" "$SUGGESTIONS_NOSPACE"

assert_true "no-space legacy box visible → returns 0" \
  tui_suggestion_box_visible "$SCREEN_WITH_BOX_NOSPACE"

SUGGESTIONS_SEP=$(tui_extract_suggestions "$SCREEN_WITH_SEPARATOR")
assert_eq "separator line excluded" "$(printf 'orders\nmetadata')" "$SUGGESTIONS_SEP"

SUGGESTIONS_OVERLAY=$(tui_extract_suggestions "$SCREEN_OVERLAY_BOX")
assert_eq "extract overlay suggestions" "$(printf '.metadata\n.orders\n.products\n.store_id\n.store_name\n.store_region\n.\nhas(\ntype\nlength\nkeys')" "$SUGGESTIONS_OVERLAY"

assert_true "overlay suggestion box present → returns 0" \
  tui_suggestion_box_visible "$SCREEN_OVERLAY_BOX"

SUGGESTIONS_POPUP=$(tui_extract_suggestions "$SCREEN_POPUP_BOX")
assert_eq "extract popup box suggestions (no space after pipe)" "$(printf 'customer\nitems\ntotals')" "$SUGGESTIONS_POPUP"

assert_true "popup box visible → returns 0" \
  tui_suggestion_box_visible "$SCREEN_POPUP_BOX"

SUGGESTIONS_POPUP_BARE=$(tui_extract_suggestions "$SCREEN_POPUP_BARE")
assert_eq "extract bare popup suggestions (no header box)" "$(printf 'customer\nitems\ntotals')" "$SUGGESTIONS_POPUP_BARE"

assert_true "bare popup visible → returns 0" \
  tui_suggestion_box_visible "$SCREEN_POPUP_BARE"

# ---------------------------------------------------------------------------
# tui_read_query
# ---------------------------------------------------------------------------

QUERY=$(tui_read_query "$SCREEN_QUERY_TEXT")
assert_eq "query bar with text" ".orders[]" "$QUERY"

QUERY_EMPTY=$(tui_read_query "$SCREEN_QUERY_EMPTY")
assert_eq "empty query bar returns empty string" "" "$QUERY_EMPTY"

# ---------------------------------------------------------------------------
# Multiple assertions from single capture (D4 spec requirement)
# ---------------------------------------------------------------------------

SCREEN="$SCREEN_WITH_BOX"
assert_true "visible check works on shared capture" \
  tui_suggestion_box_visible "$SCREEN"

SHARED_SUGG=$(tui_extract_suggestions "$SCREEN")
assert_eq "extract works on same shared capture" "$(printf 'orders\nmetadata\nuser')" "$SHARED_SUGG"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] || exit 1
