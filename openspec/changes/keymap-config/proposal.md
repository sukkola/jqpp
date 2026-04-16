## Why

`jqt`'s keybindings are hardcoded. Users who prefer different shortcuts (e.g. vim-style navigation, alternative copy keys, or ergonomic alternatives to chords like `Ctrl+Y`) have no way to remap them without recompiling. A configuration file also opens the door for other user-adjustable preferences in the future.

## What Changes

- New optional TOML configuration file (`~/.config/jqt/config.toml` by default, overridable via `--config` CLI flag) loaded at startup
- `[keys]` section maps action names to key sequences; unspecified actions keep their defaults
- All existing keybindings remain the defaults — no breaking changes for users who don't create a config file
- A `--print-config` flag prints the current effective configuration (defaults + overrides) so users have a starting point

## Capabilities

### New Capabilities

- `config-loader`: Discovers, reads, and parses the TOML config file; merges with compiled-in defaults; surfaces load errors in the footer at startup
- `keymap`: Defines the canonical action-name registry, the `KeyBinding` type (key code + modifiers), and a `Keymap` lookup table used by the event loop

### Modified Capabilities

- `tui-layout`: Key handling in the event loop changes from hardcoded comparisons to `keymap.matches(action, key_event)` lookups; footer hint strings are derived from the active keymap rather than hardcoded literals

## Impact

- New file: `src/config.rs` (config loader + TOML schema)
- New file: `src/keymap.rs` (action registry, `KeyBinding`, `Keymap`)
- Modified: `src/main.rs` — event loop replaces literal key checks with `Keymap` lookups
- Modified: `src/ui.rs` — footer hint bar reads action labels from `Keymap`
- New dependency: `toml` crate (already common in Rust ecosystem, small)
- Optional runtime file: `~/.config/jqt/config.toml` (XDG-compliant path, not required)
