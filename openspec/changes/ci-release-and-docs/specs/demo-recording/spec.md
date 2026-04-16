## ADDED Requirements

### Requirement: Example JSON fixture exists at demo/demo.json
A `demo/demo.json` file SHALL exist containing realistic nested JSON — an array of orders with nested customer, items (array of objects), and status fields — rich enough to demonstrate multi-level dot-path completion and array index completion meaningfully.

#### Scenario: JSON has nested objects and arrays
- **WHEN** jqpp loads `demo/demo.json`
- **THEN** queries like `.orders[].customer.name` and `.orders[].items[].price` are valid and return results

### Requirement: VHS tape script exists at demo/demo.tape
A `demo/demo.tape` file SHALL exist that drives jqpp through a scripted session demonstrating intellisense features with human-like pacing (no mechanical instant typing).

#### Scenario: Tape launches jqpp with demo JSON
- **WHEN** the tape runs
- **THEN** the first action is `Type "jqpp demo/demo.json"` followed by `Enter` and a `Sleep` long enough for the TUI to render

#### Scenario: Tape demonstrates dot-path field completion
- **WHEN** the user types `.` in the query bar
- **THEN** the tape shows field name suggestions appearing; the user navigates with arrow keys and selects a field with Tab

#### Scenario: Tape demonstrates nested path completion
- **WHEN** the user extends the query to a nested path (e.g., `.orders[].`)
- **THEN** suggestions for the nested object's fields appear

#### Scenario: Tape demonstrates builtin completion with type context
- **WHEN** the user types `| sel` after a pipe
- **THEN** the tape shows `select` appearing in the suggestions, reflecting type-aware filtering

#### Scenario: Tape exits cleanly
- **WHEN** the demo sequence is done
- **THEN** the tape sends `Ctrl+C` to quit and `Sleep 500ms` before ending

### Requirement: VHS tape output is a GIF
The tape SHALL declare `Output demo/demo.gif` so the output is a GIF file suitable for direct embedding in a GitHub README `<img>` tag without requiring video playback support.

#### Scenario: GIF produced at expected path
- **WHEN** `vhs demo/demo.tape` completes
- **THEN** `demo/demo.gif` exists

### Requirement: VHS tape uses legible terminal settings
The tape SHALL set `FontSize 16`, `Width 1200`, `Height 700`, and `Theme "Catppuccin Mocha"` (or equivalent dark theme available in VHS) so the TUI is readable in the README at typical display sizes.

#### Scenario: Output is readable
- **WHEN** the GIF is embedded in the README
- **THEN** the query bar, completion dropdown, and JSON output pane are all legible

### Requirement: Tape types with human-like pacing
Each `Type` command in the tape SHALL be followed by per-character sleeps or use VHS `TypingSpeed` to avoid mechanical instant appearance. Between logical steps, `Sleep 800ms`–`Sleep 1500ms` pauses SHALL be used.

#### Scenario: Typing looks natural
- **WHEN** the GIF plays back
- **THEN** characters appear at ~80–120ms intervals rather than all at once

### Requirement: mise task demo regenerates the recording
A `mise.toml` at the repo root SHALL define a task `demo` that changes to the repo root and runs `vhs demo/demo.tape`.

#### Scenario: Developer regenerates demo
- **WHEN** a developer runs `mise run demo`
- **THEN** `vhs demo/demo.tape` runs and overwrites `demo/demo.gif`
