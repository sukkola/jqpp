## Context

`jqt`'s event loop in `src/main.rs` currently matches key events with inline literal comparisons such as `key.code == KeyCode::Char('y') && key.modifiers.contains(KeyModifiers::CONTROL)`. There are roughly 15 distinct actions. The footer hint string in `src/ui.rs` is also a hardcoded literal. Adding configurability requires:

1. A data type that represents "a key chord" (code + modifiers).
2. An action registry that names each command.
3. A lookup table (`Keymap`) that maps action → binding, initialized from defaults and overridden from a config file.
4. A TOML config schema and loader.
5. Event-loop migration from literal comparisons to `keymap.matches(action, event)` calls.

The change is cross-cutting (touches `main.rs`, `ui.rs`, and adds two new modules) and introduces a new external dependency (`toml`), warranting a design doc.

## Goals / Non-Goals

**Goals:**
- User can remap any named action via `~/.config/jqt/config.toml` without recompiling
- Unspecified actions use compiled-in defaults; partial configs are valid
- Config errors (bad key syntax, unknown action name) are reported in the footer at startup — the app still launches with defaults
- `--config <path>` flag overrides the default config file location
- `--print-config` flag prints the effective TOML config (defaults + user overrides) to stdout and exits
- Footer hint bar reflects the active keymap dynamically

**Non-Goals:**
- Mouse bindings (out of scope for v1 config)
- Runtime keybinding changes without restart
- Multiple config profiles or layered config files
- Theming / color configuration in this change (separate concern)
- Windows-specific key names beyond what crossterm's `KeyCode` supports

## Decisions

### D1: TOML as config format

**Choice**: TOML with the `toml` crate (via `serde`).

**Rationale**: TOML is human-friendly for key-value tables, already idiomatic in the Rust ecosystem (used by Cargo), and maps cleanly to a `[keys]` section with `action = "Ctrl+Y"` string entries. The `toml` crate is small and widely vetted.

**Alternatives considered**: JSON (too noisy for a user-facing config file), YAML (heavier dependency, indentation-sensitive), custom INI (no off-the-shelf parser).

---

### D2: Key binding string syntax

**Choice**: Human-readable strings of the form `"Ctrl+Y"`, `"Alt+Enter"`, `"F5"`, `"/"`. Modifiers are `Ctrl`, `Alt`, `Shift` (case-insensitive); key names follow crossterm `KeyCode` names for special keys (`Enter`, `Tab`, `Esc`, `Up`, `Down`, `Left`, `Right`, `Backspace`, `Delete`, `F1`–`F12`) and bare characters for printable keys.

**Rationale**: Familiar to users of vim, tmux, and most terminal apps. Unambiguous to parse: split on `+`, last token is the key, earlier tokens are modifiers.

**Alternatives considered**: Raw crossterm `KeyCode` enum names as strings (too technical for end-users), xterm key escape sequences (not portable).

---

### D3: Action registry as a Rust enum

**Choice**: Define `pub enum Action` with one variant per named command (e.g. `Action::Submit`, `Action::CopyClipboard`, `Action::Quit`). `Keymap` is a `HashMap<Action, KeyBinding>`. The event loop calls `keymap.action_for(key_event) -> Option<Action>` and matches on the returned action variant.

**Rationale**: Enum variants are exhaustive and checked at compile time — adding a new action requires updating the match arm, preventing silent omissions. Serialization to/from string uses a separate name table (`action_name() -> &'static str`) rather than deriving, keeping the TOML key names stable regardless of internal renaming.

**Alternatives considered**: String-keyed map throughout (no compile-time exhaustiveness), trait objects per action (too much boilerplate).

---

### D4: Config file location and XDG compliance

**Choice**: Default path is `$XDG_CONFIG_HOME/jqt/config.toml`, falling back to `~/.config/jqt/config.toml`. Overridable with `--config <path>`. If the file does not exist, silently use defaults (no error). If the file exists but fails to parse, show a one-line error in the footer and continue with defaults.

**Rationale**: XDG Base Directory is the standard on Linux/macOS for non-root config files. Silent absence is the right default — new users shouldn't see config errors on first launch. Visible-but-non-fatal parse errors balance discoverability with resilience.

**Alternatives considered**: `~/.jqtrc` (non-XDG, less tidy), embedding config in the binary via `include_str!` (no user editability).

---

### D5: Footer hint bar derived from Keymap

**Choice**: `ui.rs` calls `keymap.hint_string()` which iterates a fixed ordered list of `(Action, label)` pairs and returns a formatted string like `"enter submit · tab nav · ctrl+c quit …"` using the active binding for each action.

**Rationale**: Keeps the displayed hints in sync with the active keymap — if a user remaps `Quit` to `F10`, the footer should say `f10 quit`, not `ctrl+c quit`. A fixed ordered list ensures the footer is predictable and not overly long.

**Alternatives considered**: Hardcoded footer string (doesn't reflect custom bindings), full dynamic generation (ordering unpredictable, footer could become unwieldy).

## Risks / Trade-offs

- **Key conflicts** → A user could bind two actions to the same key. Mitigation: validate on load; report all conflicts in the footer error message; reject the config and use defaults.
- **Terminal key delivery variation** → Some terminals don't deliver `Alt+` combinations or certain function keys. Mitigation: document known limitations; the app falls back gracefully if the mapped key is never received.
- **Migration effort in main.rs** → The event loop has ~15 hardcoded key comparisons spread across multiple match arms. Migrating all of them to `keymap.action_for()` is mechanical but touches many lines. Mitigation: do it in one focused commit with thorough test coverage of the new action dispatch.
- **`toml` crate dependency** → Adds a compile-time dependency. Mitigation: `toml` is tiny and already an indirect dependency of many Rust projects; it is unlikely to cause supply-chain concern.

## Migration Plan

1. Add `toml` and `serde` (with `derive` feature) to `Cargo.toml` if not already present.
2. Implement `src/keymap.rs`: `Action` enum, `KeyBinding`, `Keymap`, default table, `action_for()`, `hint_string()`.
3. Implement `src/config.rs`: TOML schema struct, file discovery, parse-and-merge logic.
4. Thread `Keymap` into `run()` (constructed once from config, passed by reference).
5. Migrate `main.rs` event loop: replace each hardcoded key comparison with a `keymap.action_for(key)` lookup and `match action { Some(Action::X) => … }`.
6. Update `ui.rs` footer to call `keymap.hint_string()`.
7. Write unit tests: default keymap round-trips, TOML override merging, conflict detection, key-string parsing.

## Open Questions

- Should `--print-config` output the full TOML including defaults, or only the user overrides? → Lean toward full effective config so users get a ready-to-edit starting point.
- Maximum number of modifier combinations to support in the parser (`Ctrl+Shift+X`): support up to two modifiers for now; three is uncommon in terminal apps.
