## ADDED Requirements

### Requirement: Page Up action is remappable
The system SHALL expose a `scroll-page-up` action in the keymap that defaults to `PageUp` and can be rebound via `~/.config/jqpp/config.toml`.

#### Scenario: Default binding active without config
- **WHEN** no keymap config is present
- **THEN** pressing PageUp in a focused pane scrolls up by one viewport height

#### Scenario: Remapped binding overrides default
- **WHEN** the user sets `scroll-page-up = "Ctrl+U"` in config
- **THEN** pressing Ctrl+U in a focused pane scrolls up by one viewport height

### Requirement: Page Down action is remappable
The system SHALL expose a `scroll-page-down` action in the keymap that defaults to `PageDown` and can be rebound via config.

#### Scenario: Default binding active without config
- **WHEN** no keymap config is present
- **THEN** pressing PageDown in a focused pane scrolls down by one viewport height

### Requirement: Home action is remappable
The system SHALL expose a `scroll-to-top` action in the keymap that defaults to `Home` and can be rebound via config.

#### Scenario: Default binding active without config
- **WHEN** no keymap config is present
- **THEN** pressing Home in a focused pane sets scroll offset to 0

### Requirement: End action is remappable
The system SHALL expose a `scroll-to-bottom` action in the keymap that defaults to `End` and can be rebound via config.

#### Scenario: Default binding active without config
- **WHEN** no keymap config is present
- **THEN** pressing End in a focused pane sets scroll offset to maximum

### Requirement: New actions appear in --print-config output
The system SHALL include `scroll-page-up`, `scroll-page-down`, `scroll-to-top`, and `scroll-to-bottom` in the output of `jqpp --print-config`.

#### Scenario: All four actions listed
- **WHEN** the user runs `jqpp --print-config`
- **THEN** all four new action names appear with their current bindings

### Requirement: README keybindings table documents new keys
The README keybindings table SHALL include entries for Page Up, Page Down, Home, and End with descriptions of their effect.

#### Scenario: Table entries present
- **WHEN** a user reads the README keybindings section
- **THEN** they see `Page Up`, `Page Down`, `Home`, and `End` listed with accurate descriptions
