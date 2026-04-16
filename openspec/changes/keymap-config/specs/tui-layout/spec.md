## MODIFIED Requirements

### Requirement: Ctrl+Y copies focused pane content to clipboard
The application SHALL copy context-appropriate content to the clipboard when the action bound to `CopyClipboard` (default: `Ctrl+Y`) is triggered, based on which pane is focused.

#### Scenario: Copy query from query input
- **WHEN** the user triggers `CopyClipboard` with the query input focused
- **THEN** the query string is written to the clipboard

#### Scenario: Copy raw input from left pane
- **WHEN** the user triggers `CopyClipboard` with the left JSON pane focused
- **THEN** the raw input JSON bytes are written to the clipboard

#### Scenario: Copy output from right pane
- **WHEN** the user triggers `CopyClipboard` with the right output pane focused and no error is shown
- **THEN** the formatted query result is written to the clipboard

#### Scenario: Copy error text from right pane
- **WHEN** the user triggers `CopyClipboard` with the right output pane focused and an error is displayed
- **THEN** the error message text is copied

### Requirement: Footer hint bar reflects active keymap
The footer SHALL derive its keybinding hint string from the active `Keymap` rather than a hardcoded literal, so that user-configured bindings are shown correctly.

#### Scenario: Default footer unchanged when no config
- **WHEN** no config file is present
- **THEN** the footer hint string matches the existing default text

#### Scenario: Remapped action shown in footer
- **WHEN** a user remaps `Action::Quit` to `"F10"` in their config
- **THEN** the footer shows `f10 quit` instead of `ctrl+c quit`

### Requirement: Q or Ctrl+C exits
The application SHALL exit cleanly when the action bound to `Quit` (default: `Ctrl+C` / `q`) is triggered, or when `q` is pressed while focus is not on the query input.

#### Scenario: Quit with default binding
- **WHEN** the user triggers the `Quit` action from any state
- **THEN** the application exits with code 0

#### Scenario: Quit with remapped binding
- **WHEN** the user has remapped `Quit` to `"F10"` and presses F10
- **THEN** the application exits with code 0
