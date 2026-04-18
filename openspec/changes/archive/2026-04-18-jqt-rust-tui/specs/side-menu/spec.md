## ADDED Requirements

### Requirement: Side menu hidden by default
The side-menu column SHALL not be rendered on startup. The layout SHALL use the full width for the two-pane (JSON input + output) view.

#### Scenario: No menu on launch
- **WHEN** `jqt` starts
- **THEN** no side-menu column is visible and the two panes occupy the full terminal width

### Requirement: Ctrl+M toggles side-menu visibility
Pressing Ctrl+M SHALL show the side-menu column if hidden, or hide it if visible.

#### Scenario: Open menu
- **WHEN** the side-menu is hidden and user presses Ctrl+M
- **THEN** the side-menu column appears on the left; the two panes shrink to accommodate it

#### Scenario: Close menu
- **WHEN** the side-menu is visible and user presses Ctrl+M
- **THEN** the side-menu column disappears and the two panes expand back to full width

### Requirement: Side-menu renders stub item list
When visible, the side-menu SHALL render the following static items in order: Query, Config, Runs, Output, History, Saved. One item is always highlighted.

#### Scenario: Items rendered
- **WHEN** the side-menu is visible
- **THEN** all six item labels are shown; the currently selected item is highlighted

### Requirement: Arrow keys navigate menu items when menu has focus
When the side-menu is focused, Up/Down arrow keys SHALL move the highlight. Items perform no action in v1.

#### Scenario: Down arrow moves highlight
- **WHEN** the side-menu is focused and user presses Down arrow
- **THEN** the highlight moves to the next item; wraps to the first item from the last

#### Scenario: Up arrow moves highlight
- **WHEN** the side-menu is focused and user presses Up arrow
- **THEN** the highlight moves to the previous item; wraps to the last item from the first

### Requirement: Side-menu panel contains no content
The side-menu panel body SHALL be empty (no content widget) in v1. Only the item list is rendered.

#### Scenario: Empty panel body
- **WHEN** any side-menu item is selected
- **THEN** no content is shown in the panel body; selecting an item has no effect beyond moving the highlight
