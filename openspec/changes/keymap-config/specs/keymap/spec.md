## ADDED Requirements

### Requirement: Action enum covers all named commands
The `Action` enum SHALL have one variant for every remappable command in the application. The canonical action names used in TOML config files SHALL be stable kebab-case strings independent of the Rust variant names.

#### Scenario: All actions enumerable
- **WHEN** `Action::all()` is called
- **THEN** it returns every variant exactly once with no duplicates

#### Scenario: TOML name round-trip
- **WHEN** an action is converted to its TOML name string and back
- **THEN** the original variant is recovered

### Requirement: KeyBinding represents a key code and optional modifiers
`KeyBinding` SHALL store a crossterm `KeyCode` and a `KeyModifiers` bitmask. It SHALL implement `Display` for the human-readable string form (e.g. `"Ctrl+Y"`, `"F5"`, `"/"`) used in footer hints.

#### Scenario: Display for chord with modifier
- **WHEN** a `KeyBinding` has `KeyCode::Char('y')` and `KeyModifiers::CONTROL`
- **THEN** its `Display` output is `"Ctrl+Y"`

#### Scenario: Display for bare character
- **WHEN** a `KeyBinding` has `KeyCode::Char('q')` and no modifiers
- **THEN** its `Display` output is `"q"`

#### Scenario: Display for special key
- **WHEN** a `KeyBinding` has `KeyCode::Enter` and no modifiers
- **THEN** its `Display` output is `"Enter"`

### Requirement: Key string parser converts human-readable strings to KeyBinding
A `parse_key_binding(s: &str) -> Result<KeyBinding, String>` function SHALL accept strings of the form `"Ctrl+Y"`, `"Alt+Enter"`, `"F5"`, `"/"` and return the corresponding `KeyBinding`. Modifier names SHALL be case-insensitive.

#### Scenario: Ctrl modifier parsed
- **WHEN** the string `"ctrl+y"` is parsed
- **THEN** the result is `KeyCode::Char('y')` + `CONTROL`

#### Scenario: Multiple modifiers parsed
- **WHEN** the string `"Ctrl+Shift+S"` is parsed
- **THEN** the result is `KeyCode::Char('s')` + `CONTROL | SHIFT`

#### Scenario: Special key name parsed
- **WHEN** the string `"F5"` is parsed
- **THEN** the result is `KeyCode::F(5)` with no modifiers

#### Scenario: Invalid string rejected
- **WHEN** the string `"Ctrl+"` (missing key) is parsed
- **THEN** an error is returned

### Requirement: Keymap maps actions to bindings and supports event lookup
`Keymap` SHALL store one `KeyBinding` per `Action` and expose:
- `action_for(event: &KeyEvent) -> Option<Action>`: returns the action whose binding matches the event, or `None`
- `binding_for(action: Action) -> &KeyBinding`: returns the binding for a given action
- `hint_string() -> String`: returns a short footer-style string listing a curated subset of actions with their active bindings

#### Scenario: Matching event returns action
- **WHEN** a key event matching the binding for `Action::Quit` arrives
- **THEN** `action_for` returns `Some(Action::Quit)`

#### Scenario: Non-matching event returns None
- **WHEN** a key event that matches no binding arrives
- **THEN** `action_for` returns `None`

#### Scenario: Hint string reflects active bindings
- **WHEN** `Action::CopyClipboard` is remapped to `"Alt+C"` in the keymap
- **THEN** the hint string contains `"alt+c"` rather than the default `"ctrl+y"`

### Requirement: Default keymap matches current hardcoded bindings
`Keymap::default()` SHALL produce bindings identical to the current hardcoded key checks so that users without a config file experience no behaviour change.

#### Scenario: Default quit binding
- **WHEN** the default keymap is constructed
- **THEN** `Action::Quit` is bound to `Ctrl+C`

#### Scenario: Default submit binding
- **WHEN** the default keymap is constructed
- **THEN** `Action::Submit` is bound to `Enter`
