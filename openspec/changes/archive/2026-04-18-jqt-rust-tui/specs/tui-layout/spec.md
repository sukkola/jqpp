## ADDED Requirements

### Requirement: Two-pane layout with query input
The application SHALL render a full-screen terminal UI with: a top query-input bar, a left JSON-input pane, a right output pane, and a footer keybinding bar. The side-menu column SHALL be hidden by default.

#### Scenario: Initial render
- **WHEN** the application starts
- **THEN** only the query-input bar, left JSON pane, right output pane, and footer are visible; no side-menu column is shown

#### Scenario: Footer shows keybindings
- **WHEN** the application is in any state
- **THEN** the footer SHALL display: `enter submit query · tab section · ctrl+y copy query · ctrl+t hide input · ctrl+s save output · ctrl+m menu`

### Requirement: Tab cycles focus between panes
The application SHALL cycle focus through: query input → left pane → right pane → query input on each Tab press.

#### Scenario: Tab from query input
- **WHEN** focus is on the query-input bar and user presses Tab
- **THEN** focus moves to the left JSON pane

#### Scenario: Tab from right pane wraps
- **WHEN** focus is on the right output pane and user presses Tab
- **THEN** focus returns to the query-input bar

### Requirement: Ctrl+T toggles query input visibility
The application SHALL show or hide the top query-input bar on Ctrl+T.

#### Scenario: Hide input
- **WHEN** user presses Ctrl+T and the input bar is visible
- **THEN** the input bar is hidden and the panes expand to fill the space

#### Scenario: Show input
- **WHEN** user presses Ctrl+T and the input bar is hidden
- **THEN** the input bar becomes visible again

### Requirement: Ctrl+S saves output to file
The application SHALL write the current output pane content to a file on Ctrl+S.

#### Scenario: Save output
- **WHEN** user presses Ctrl+S
- **THEN** the output content is written to `jqt-output.json` in the current directory and a confirmation message appears in the footer

### Requirement: Ctrl+Y copies focused pane content to clipboard
The application SHALL copy context-appropriate content to the clipboard on Ctrl+Y based on which pane is focused.

#### Scenario: Copy query from query input
- **WHEN** user presses Ctrl+Y with the query input focused
- **THEN** the query string is written to the clipboard

#### Scenario: Copy raw input from left pane
- **WHEN** user presses Ctrl+Y with the left JSON pane focused
- **THEN** the raw input JSON bytes are written to the clipboard

#### Scenario: Copy output from right pane
- **WHEN** user presses Ctrl+Y with the right output pane focused and no error is shown
- **THEN** the formatted query result is written to the clipboard

#### Scenario: Copy error text from right pane
- **WHEN** user presses Ctrl+Y with the right output pane focused and an error is displayed
- **THEN** the error message text is copied (not an empty string)

### Requirement: Q or Ctrl+C exits
The application SHALL exit cleanly on `q` (when focus is not on the query input) or `Ctrl+C` at any time.

#### Scenario: Quit with q
- **WHEN** focus is on the left or right pane and user presses `q`
- **THEN** the application exits with code 0

#### Scenario: Quit with Ctrl+C
- **WHEN** user presses Ctrl+C in any state
- **THEN** the application exits with code 0
