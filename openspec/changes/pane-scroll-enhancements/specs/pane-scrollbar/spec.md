## ADDED Requirements

### Requirement: Scrollbar rendered on input and output panes
The system SHALL render a vertical scrollbar on the right edge of the input pane and the output pane whenever the content exceeds the visible pane height.

#### Scenario: Scrollbar visible when content overflows
- **WHEN** the content in a pane has more lines than the pane can display
- **THEN** a scrollbar is rendered on the rightmost column of that pane

#### Scenario: Scrollbar thumb position reflects scroll offset
- **WHEN** the user has scrolled a pane down by some amount
- **THEN** the scrollbar thumb is positioned proportionally to the current scroll offset within the total content height

#### Scenario: No scrollbar when content fits
- **WHEN** all content lines fit within the visible pane area
- **THEN** no scrollbar is rendered at all — the pane uses the full width

### Requirement: Scrollbar click jumps to position
The system SHALL respond to a left mouse button click on the scrollbar track by immediately updating the pane's scroll offset to the proportional position of the click within the track.

#### Scenario: Click on upper track area
- **WHEN** the user clicks the left mouse button on the scrollbar column within the pane's row range
- **THEN** the pane scroll offset is set to `(click_row - pane_top) * content_lines / pane_height`

#### Scenario: Click outside scrollbar column does nothing
- **WHEN** the user clicks in a column that is not the scrollbar column
- **THEN** the scroll offset is unchanged

### Requirement: Scrollbar drag scrolls continuously
The system SHALL update the pane's scroll offset continuously as the user drags the mouse after pressing the left mouse button on the scrollbar column.

#### Scenario: Drag downward increases scroll offset
- **WHEN** the user presses and holds the left mouse button on the scrollbar column and moves the pointer downward
- **THEN** the scroll offset increases proportionally to the drag position on each drag event

#### Scenario: Drag upward decreases scroll offset
- **WHEN** the user drags the pointer upward while holding the mouse button on the scrollbar column
- **THEN** the scroll offset decreases proportionally

#### Scenario: Releasing mouse ends drag
- **WHEN** the user releases the mouse button
- **THEN** no further drag-based scroll updates occur

### Requirement: Mouse wheel scrolls the pane under the pointer
The system SHALL scroll the pane currently under the mouse pointer when a mouse wheel or trackpad scroll event is received, regardless of which pane has keyboard focus.

#### Scenario: Wheel event inside right pane
- **WHEN** a mouse scroll-down event occurs and the pointer row is within the output pane's row range
- **THEN** `right_scroll` is incremented by 1

#### Scenario: Wheel event inside left pane
- **WHEN** a mouse scroll-up event occurs and the pointer row is within the input pane's row range
- **THEN** `left_scroll` is decremented by 1 (saturating at 0)

#### Scenario: Scroll offset clamped at maximum
- **WHEN** a scroll event would push the offset past the last content line
- **THEN** the offset is clamped to `content_lines.saturating_sub(pane_height)`
