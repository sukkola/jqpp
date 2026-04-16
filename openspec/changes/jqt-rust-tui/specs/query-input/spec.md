## ADDED Requirements

### Requirement: Single-line query input with prompt
The query-input widget SHALL render a single-line text editor inside a titled border block.

#### Scenario: Query bar visible
- **WHEN** the application renders the query-input bar
- **THEN** a bordered block titled ` Query ` is displayed at the top of the screen

### Requirement: Ghost-text inline completion
The query-input SHALL display a ghost-text suffix (dimmed/gray) representing the top completion suggestion. The ghost text starts immediately after the last typed character.

#### Scenario: Ghost text appears
- **WHEN** a completion provider returns at least one suggestion whose insert_text extends beyond the current input
- **THEN** the remaining suffix is shown in `Color::DarkGray` style after the cursor

#### Scenario: No ghost text when no suggestions match
- **WHEN** no suggestion's insert_text starts with the current input
- **THEN** no ghost text is rendered; the textarea is drawn normally

### Requirement: Suggestion dropdown
When suggestions are available, the application SHALL render a floating dropdown list anchored to the cursor column, overlapping the query bar's bottom border. It has left + right + bottom borders only (the query bar's bottom border acts as the visual top edge).

#### Scenario: Dropdown appears
- **WHEN** `show_suggestions` is true and at least one suggestion exists
- **THEN** a dropdown is rendered starting at `query_area.y + query_area.height - 1` at the cursor column

#### Scenario: Dropdown height capped
- **WHEN** more than 11 suggestions are available
- **THEN** the dropdown height is capped at 12 rows (11 items + 1 bottom border)

#### Scenario: Dropdown hidden at screen edge
- **WHEN** the cursor is at or beyond the screen width
- **THEN** no dropdown is rendered

#### Scenario: Down arrow moves selection
- **WHEN** the dropdown is visible and user presses Down
- **THEN** `suggestion_index` increments (wrapping) and ghost text updates

#### Scenario: Up arrow at top dismisses dropdown
- **WHEN** the dropdown is visible and the first item is selected and user presses Up
- **THEN** the dropdown closes; `suggestion_active` is cleared

#### Scenario: Esc closes dropdown
- **WHEN** the dropdown is visible and user presses Esc
- **THEN** the dropdown closes; the typed value is retained

### Requirement: Tab and Enter accept the highlighted suggestion
Pressing Tab or Enter when the dropdown is visible SHALL replace the query with the full `insert_text` of the highlighted suggestion and position the cursor correctly.

#### Scenario: Accept suggestion — no parameters
- **WHEN** the accepted `insert_text` contains no `(`
- **THEN** the cursor is placed at the end of the inserted text

#### Scenario: Accept suggestion — string parameter
- **WHEN** the accepted `insert_text` contains `("`
- **THEN** the cursor is placed immediately after `("` so the user can type the argument directly

#### Scenario: Accept suggestion — non-string parameter
- **WHEN** the accepted `insert_text` contains `(` but not `("`
- **THEN** the cursor is placed immediately after `(`

#### Scenario: Tab with no suggestion cycles focus
- **WHEN** no dropdown is visible and user presses Tab
- **THEN** focus moves to the next pane

### Requirement: Enter submits query when dropdown is not visible
Pressing Enter without an active dropdown SHALL execute the current query.

#### Scenario: Submit query
- **WHEN** user presses Enter with no dropdown visible
- **THEN** the query is evaluated and the result appears in the output pane; the query is pushed to history

### Requirement: Up/Down arrow opens context-based suggestions
Pressing Down (or Up) when the dropdown is not visible SHALL open the suggestion dropdown.

#### Scenario: Down opens dropdown from cache
- **WHEN** dropdown is not visible and cached suggestions exist
- **THEN** `show_suggestions` becomes true immediately

#### Scenario: Down triggers re-computation when no cache
- **WHEN** dropdown is not visible and suggestion list is empty
- **THEN** a debounce tick is forced to compute fresh suggestions

### Requirement: Backspace always re-arms completions
Pressing Backspace or Delete SHALL set `suggestion_active = true` so the debounce loop recomputes suggestions even while erasing characters mid-word.

#### Scenario: Backspace mid-word re-triggers suggestions
- **WHEN** user presses Backspace while partially through a function name
- **THEN** suggestions update to reflect the shorter prefix

### Requirement: Up/Down arrow navigates history when dropdown is closed
When no dropdown is visible, Up moves to the previous submitted query (in reverse order).

#### Scenario: Navigate to previous query
- **WHEN** user presses Up with no dropdown visible and history is non-empty
- **THEN** the input is replaced with the previous submitted query

### Requirement: Suggestion dropout has no "Suggestions" heading
The dropdown SHALL NOT render any heading or title text. The query bar's bottom border serves as the visual top of the dropdown.

#### Scenario: No heading in rendered output
- **WHEN** the dropdown is rendered
- **THEN** the string "Suggestions" does not appear anywhere in the terminal buffer

### Requirement: Rolling-window scroll for suggestion dropdown
When the suggestion list exceeds the visible dropdown height (11 items), the widget SHALL maintain a `suggestion_scroll` offset so that the highlighted item is always visible.

#### Scenario: Scroll advances when selection leaves bottom
- **WHEN** `suggestion_index` moves past the last visible slot
- **THEN** `suggestion_scroll` increments so the selected item remains in view

#### Scenario: Scroll retreats when selection moves above window
- **WHEN** `suggestion_index` is above `suggestion_scroll`
- **THEN** `suggestion_scroll` decreases to the index position

#### Scenario: No scrolling for small lists
- **WHEN** the suggestion list has fewer items than `DROPDOWN_VISIBLE`
- **THEN** `suggestion_scroll` stays at 0

### Requirement: @ character triggers format-operator suggestions
Typing `@` SHALL arm `suggestion_active` and surface format-operator completions (`@base64`, `@csv`, `@tsv`, `@uri`, `@html`, `@sh`, `@json`, `@text`) from the builtin catalog.

#### Scenario: @ shows format operators
- **WHEN** the user types `|@` (or just `@` after a pipe)
- **THEN** `@`-prefixed completions appear in the dropdown

### Requirement: Bracketed paste bypasses per-character intellisense
When the terminal supports bracketed paste mode, pasting a string SHALL be handled as a single `Event::Paste` event rather than individual keystrokes. This prevents the suggestion pipeline from being invoked for every character.

#### Scenario: Paste does not thrash suggestions
- **WHEN** the user pastes a multi-character string
- **THEN** a single debounce tick fires after the full paste, not one per character

### Requirement: Double-Esc clears the query bar
Pressing Esc twice within 500 ms while the query bar is focused SHALL clear the textarea content and fire an immediate debounce.

#### Scenario: First Esc dismisses dropdown
- **WHEN** the dropdown is visible and the user presses Esc
- **THEN** the dropdown closes; the typed value is retained

#### Scenario: Second Esc within 500 ms clears query
- **WHEN** the user presses Esc a second time within 500 ms with no dropdown visible
- **THEN** the textarea is cleared and the debounce fires immediately

### Requirement: Mid-query completions use cursor position
When the user moves the cursor into the middle of an existing query and types, completion suggestions SHALL be computed from the text to the left of the cursor only, not the full query string.

#### Scenario: Correct suggestions when cursor is mid-query
- **WHEN** the cursor is positioned after `| ` in `.config | sort | .name` (between the two pipes)
- **THEN** the token used for completion is `sort`, not `.name`

#### Scenario: Accept suggestion preserves text after cursor
- **WHEN** the user accepts a suggestion while the cursor is mid-query
- **THEN** the text to the right of the cursor is appended after the accepted `insert_text`
