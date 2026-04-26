#!/usr/bin/env bash
# screenshot.sh — Screen-parsing library for agent-tui --strip-ansi output.
#
# Source this file to get three functions:
#   tui_suggestion_box_visible  SCREEN  → exit 0 if suggestion box present, 1 if not
#   tui_extract_suggestions     SCREEN  → prints suggestion labels, one per line
#   tui_read_query              SCREEN  → prints current query bar text
#
# All functions accept the screen content as a variable argument so a single
# capture can be passed to multiple functions without re-running agent-tui:
#
#   SCREEN=$(agent-tui -s "$SESSION" screenshot --strip-ansi)
#   tui_suggestion_box_visible "$SCREEN"
#   tui_extract_suggestions    "$SCREEN"
#   tui_read_query             "$SCREEN"
#
# Screen layout (jqpp with suggestion box open):
#
#   ┌──────────── Query ─────────┐
#   │ .                          │   ← query content line
#   ├────────────────────────────┤   ← separator; ONLY present when box open
#   │ orders                     │   ← suggestion rows
#   │ metadata                   │
#   └────────────────────────────┘
#   ┌──────────── Output ────────┐
#   │ ...                        │
#   └────────────────────────────┘

_tui_extract_suggestions_legacy() {
  local screen="$1"
  # Extract content after the first │ on each suggestion line. This handles both
  # │ word │ (content between two pipes) and │word (no closing pipe) formats.
  echo "$screen" \
    | awk '
      /^[[:space:]]*├[─]+/ { found=1; next }
      found && /^[[:space:]]*└[─]+/ { exit }
      found && /│/ {
        n = split($0, a, "│")
        if (n >= 2) {
          s = a[2]
          gsub(/^[[:space:]]+|[[:space:]]+$/, "", s)
          if (length(s) == 0) next
          if (s ~ /^[─┄┅┈┉-]+$/) next
          print s
        }
      }
    '
}

_tui_extract_suggestions_overlay() {
  local screen="$1"
  # Newer UI overlays the dropdown over the query/input border without a dedicated
  # ├ separator row. In this mode suggestions appear as a small left box:
  #   └│first        │...
  #    │second       │...
  #    │third        │...
  #    └─────────────┘
  echo "$screen" \
    | awk '
      function emit_candidate(raw,    s) {
        s = raw
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", s)
        if (length(s) == 0 || length(s) > 32) return
        if (s ~ /^[─┄┅┈┉-]+$/) return
        if (s ~ /^[█▌▐▉▊▋▍▎▏║]+$/) return
        if (s ~ /["{},:]/) return
        if (s ~ /^(Query|Input|Output)$/) return
        if (s ~ /^[._$[:alnum:]\[\]()]+$/) print s
      }

      {
        if (!in_box) {
          if (index($0, "└") > 0 && index($0, "│") > 0) {
            n = split($0, a, "│")
            if (n >= 2) {
              for (i = 2; i <= n; i++) emit_candidate(a[i])
              in_box = 1
            }
          }
          next
        }

        if ($0 ~ /^[[:space:]]*│/ || $0 ~ /^[[:space:]]*┌[^│]*│/) {
          n = split($0, a, "│")
          if (n >= 2) {
            for (i = 2; i <= n; i++) emit_candidate(a[i])
          }
          next
        }

        if ($0 ~ /^[[:space:]]*└[─]+┘/) {
          in_box = 0
          next
        }

        # Any non-row line ends this overlay block.
        in_box = 0
      }
    '
}

# Handles suggestion popups that appear as a standalone box BELOW the query bar,
# separated from it by a clean └───┘ line (no │ on the same line).
# Layout example:
#   └─────────────────────────────────────────────────────┘
#   ┌──────────────┐     OR:   │ customer                 │
#   │ customer     │           │ items                    │
#   │ items        │           │ totals                   │
#   └──────────────┘
#    ┌ Input ───────...
_tui_extract_suggestions_popup() {
  local screen="$1"
  echo "$screen" \
    | awk '
      function emit_candidate(raw,    s) {
        s = raw
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", s)
        if (length(s) == 0 || length(s) > 32) return
        if (s ~ /^[─┄┅┈┉-]+$/) return
        if (s ~ /^[█▌▐▉▊▋▍▎▏║]+$/) return
        if (s ~ /["{},:]/) return
        if (s ~ /^(Query|Input|Output|Screenshot)$/) return
        if (s ~ /^[._$[:alnum:]\[\]()]+$/) print s
      }
      /Query/ { in_query = 1; next }
      in_query && /└[─]/ && !/│/ { in_query = 0; after_query = 1; next }
      # Any named pane header (┌── text ──┐) ends the zone.
      # Unnamed box borders (┌──────────┐) are suggestion popups and do NOT exit.
      after_query && /┌[─ ]*[[:alpha:]]/ { exit }
      after_query && /Screenshot/ { exit }
      after_query {
        # Skip lines that are just box borders
        if ($0 ~ /^[[:space:]]*┌[─]+┐[[:space:]]*$/ || $0 ~ /^[[:space:]]*└[─]+┘[[:space:]]*$/) next
        n = split($0, a, "│")
        if (n >= 2) {
          for (i = 2; i <= n; i++) emit_candidate(a[i])
        }
      }
    '
}

# tui_suggestion_box_visible SCREEN
#
# Returns 0 if a suggestion box is visible, 1 otherwise.
tui_suggestion_box_visible() {
  local screen="$1"
  [ -n "$(tui_extract_suggestions "$screen")" ]
}

# tui_extract_suggestions SCREEN
#
# Prints one suggestion label per line, trimmed of whitespace.
# Extracts only lines that appear in the suggestion block (after the ├ separator,
# before the next └ closing row). Excludes blank lines and separator-only rows.
tui_extract_suggestions() {
  local screen="$1"
  local out

  out=$(_tui_extract_suggestions_legacy "$screen")
  if [ -n "$out" ]; then
    printf '%s\n' "$out"
    return
  fi

  out=$(_tui_extract_suggestions_overlay "$screen")
  if [ -n "$out" ]; then
    printf '%s\n' "$out"
    return
  fi

  _tui_extract_suggestions_popup "$screen"
}

# tui_read_query SCREEN
#
# Returns the current text in the Query input bar, trimmed.
# The Query bar content is the first │...│ line after the ┌─── Query ───┐ header.
tui_read_query() {
  local screen="$1"
  echo "$screen" \
    | awk '/Query/{found=1; next} found && /│/{print; exit}' \
    | sed 's/│//g' \
    | sed 's/^[[:space:]]*//; s/[[:space:]]*$//'
}
