## 1. Dependencies and Module Scaffolding

- [x] 1.1 Add `toml = { version = "0.8", features = ["parse"] }` and ensure `serde` with `derive` feature is in `Cargo.toml`
- [x] 1.2 Create empty `src/keymap.rs` and `src/config.rs` and register them in `src/main.rs` with `mod keymap; mod config;`

## 2. Action Enum and KeyBinding

- [x] 2.1 Define `pub enum Action` in `src/keymap.rs` with variants for all remappable commands: `Submit`, `AcceptSuggestion`, `NextPane`, `PrevPane`, `Quit`, `CopyClipboard`, `SaveOutput`, `ToggleQueryBar`, `ToggleMenu`, `HistoryUp`, `HistoryDown`, `SuggestionUp`, `SuggestionDown`, `ScrollUp`, `ScrollDown`
- [x] 2.2 Implement `Action::all() -> &'static [Action]` returning every variant
- [x] 2.3 Implement `Action::toml_name(&self) -> &'static str` returning stable kebab-case strings (e.g. `"quit"`, `"copy-clipboard"`)
- [x] 2.4 Implement `Action::from_toml_name(s: &str) -> Option<Action>`
- [x] 2.5 Define `pub struct KeyBinding { code: KeyCode, modifiers: KeyModifiers }`
- [x] 2.6 Implement `Display for KeyBinding` producing strings like `"Ctrl+Y"`, `"Enter"`, `"q"`
- [x] 2.7 Implement `pub fn parse_key_binding(s: &str) -> Result<KeyBinding, String>`: split on `+`, parse modifier tokens (`ctrl`, `alt`, `shift` case-insensitive), parse final token as `KeyCode`
- [x] 2.8 Write unit tests: modifier parsing, special key names (`Enter`, `Tab`, `Esc`, `Up`, `Down`, `F1`–`F12`, `Backspace`, `Delete`), bare characters, multiple modifiers, invalid strings rejected

## 3. Keymap

- [x] 3.1 Define `pub struct Keymap(HashMap<Action, KeyBinding>)` (requires `Action: Hash + Eq`)
- [x] 3.2 Implement `Keymap::default()` with hardcoded defaults matching all current key checks in `main.rs`
- [x] 3.3 Implement `Keymap::action_for(&self, event: &KeyEvent) -> Option<Action>`: match event code + modifiers against stored bindings
- [x] 3.4 Implement `Keymap::binding_for(&self, action: Action) -> &KeyBinding`
- [x] 3.5 Implement `Keymap::hint_string(&self) -> String`: iterate a fixed curated list of `(Action, short_label)` pairs and format with active bindings (e.g. `"enter submit · tab nav · ctrl+c quit …"`)
- [x] 3.6 Write unit tests: default bindings round-trip, `action_for` match/no-match, `hint_string` reflects remapped action

## 4. Config Loader

- [x] 4.1 Define TOML schema struct in `src/config.rs`: `#[derive(Deserialize, Default)] struct Config { #[serde(default)] keys: HashMap<String, String> }`
- [x] 4.2 Implement `fn resolve_config_path(override_path: Option<&Path>) -> Option<PathBuf>`: check `--config` arg, then `$XDG_CONFIG_HOME/jqt/config.toml`, then `~/.config/jqt/config.toml`
- [x] 4.3 Implement `fn load_keymap(override_path: Option<&Path>) -> (Keymap, Option<String>)`: returns merged `Keymap` and an optional error/warning string for the footer
- [x] 4.4 Handle absent file: return `(Keymap::default(), None)`
- [x] 4.5 Handle TOML parse error: return `(Keymap::default(), Some("Config error: …"))`
- [x] 4.6 Handle unknown action names: skip entry, accumulate warnings, apply known entries
- [x] 4.7 Handle unparseable key string: skip entry, accumulate warnings, apply valid entries
- [x] 4.8 Implement conflict detection: after merging all overrides, check for duplicate bindings across actions; on conflict return `(Keymap::default(), Some("Config conflict: …"))`
- [x] 4.9 Write unit tests: absent file returns defaults, partial override merges correctly, invalid TOML returns defaults + error string, unknown action warns, conflict detected and rejected

## 5. CLI Integration

- [x] 5.1 Add `--config <path>` optional argument to `clap` `Args` struct
- [x] 5.2 Add `--print-config` boolean flag to `Args`
- [x] 5.3 In `actual_main`: if `--print-config`, call `load_keymap`, serialize effective config to TOML, print to stdout, exit 0
- [x] 5.4 In `actual_main`: call `load_keymap(args.config.as_deref())` before entering `run()`; pass `keymap` and optional config error string into `run()`

## 6. Event Loop Migration

- [x] 6.1 Thread `keymap: &Keymap` through the `run()` function signature
- [x] 6.2 At top of key-event handling, compute `let action = keymap.action_for(&key)` and replace each hardcoded key comparison block with a `match action { Some(Action::X) => … }` arm
- [x] 6.3 Preserve `is_ctrl_quit` / `is_pane_quit` semantics: `Quit` action covers both `Ctrl+C` variants; bare `q` in non-QueryInput state should remain a separate check or be its own action (`Action::QuitPane`)
- [x] 6.4 Ensure the `AppState::QueryInput` branch still passes unrecognised keys through to `tui-textarea` (only intercept keys that map to a known action)
- [x] 6.5 If a config error string was returned by `load_keymap`, show it in the footer for 5 seconds at startup

## 7. Footer Update

- [x] 7.1 Pass `keymap: &Keymap` into `ui::draw()`
- [x] 7.2 Replace the hardcoded footer hint string with `keymap.hint_string()`
- [x] 7.3 Update `ui.rs` unit test that checks footer content to use the default hint string derived from `Keymap::default()`

## 8. Tests and Documentation

- [x] 8.1 Integration test: construct a TOML config string remapping one action, load it, verify `action_for` returns the new binding
- [x] 8.2 Integration test: conflicting config returns defaults and non-empty error string
- [x] 8.3 Update `README.md`: add Configuration section documenting the config file location, `[keys]` syntax, action name table, and `--print-config` flag
- [x] 8.4 Verify `cargo test` passes for all tests
- [x] 8.5 Verify `cargo clippy -- -D warnings` passes
