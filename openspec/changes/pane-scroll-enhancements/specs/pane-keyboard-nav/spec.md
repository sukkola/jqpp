## ADDED Requirements

### Requirement: Page Down scrolls by one viewport height
The system SHALL scroll the focused pane down by one viewport height when the Page Down key (or remapped equivalent) is pressed while the left or right pane has focus.

#### Scenario: Page Down in output pane
- **WHEN** the output pane has focus and the user presses Page Down
- **THEN** `right_scroll` increases by `right_pane_height`, clamped to the maximum scroll offset

#### Scenario: Page Down in input pane
- **WHEN** the input pane has focus and the user presses Page Down
- **THEN** `left_scroll` increases by `left_pane_height`, clamped to the maximum scroll offset

#### Scenario: Page Down ignored in query bar
- **WHEN** the query bar has focus and the user presses Page Down
- **THEN** the key event is not consumed and scroll offsets are unchanged

### Requirement: Page Up scrolls back by one viewport height
The system SHALL scroll the focused pane up by one viewport height when the Page Up key (or remapped equivalent) is pressed while the left or right pane has focus.

#### Scenario: Page Up in output pane
- **WHEN** the output pane has focus and the user presses Page Up
- **THEN** `right_scroll` decreases by `right_pane_height`, saturating at 0

#### Scenario: Page Up from top does not underflow
- **WHEN** the pane is already at scroll offset 0 and the user presses Page Up
- **THEN** scroll offset stays at 0 (no underflow)

### Requirement: Home jumps to the top of the pane
The system SHALL set the focused pane's scroll offset to 0 when the Home key (or remapped equivalent) is pressed while the left or right pane has focus.

#### Scenario: Home in output pane
- **WHEN** the output pane has focus and the user presses Home
- **THEN** `right_scroll` is set to 0

#### Scenario: Home in input pane
- **WHEN** the input pane has focus and the user presses Home
- **THEN** `left_scroll` is set to 0

### Requirement: End jumps to the bottom of the pane
The system SHALL set the focused pane's scroll offset to the maximum value (last line visible) when the End key (or remapped equivalent) is pressed while the left or right pane has focus.

#### Scenario: End in output pane
- **WHEN** the output pane has focus and the user presses End
- **THEN** `right_scroll` is set to `right_content_lines.saturating_sub(right_pane_height)`

#### Scenario: End when content fits entirely
- **WHEN** all content fits within the pane and the user presses End
- **THEN** `right_scroll` remains 0 (no negative offset)
