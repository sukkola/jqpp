## ADDED Requirements

### Requirement: Discover and load TOML config file at startup
The config loader SHALL look for a TOML config file at `$XDG_CONFIG_HOME/jqt/config.toml`, falling back to `~/.config/jqt/config.toml` when `XDG_CONFIG_HOME` is not set. The `--config <path>` CLI flag SHALL override the default location.

#### Scenario: Config file absent
- **WHEN** no config file exists at the resolved path
- **THEN** the application starts normally with compiled-in defaults; no error or warning is shown

#### Scenario: Config file present and valid
- **WHEN** a valid TOML config file exists at the resolved path
- **THEN** its values are merged over the defaults and the application starts with the merged keymap

#### Scenario: --config flag overrides default path
- **WHEN** the user launches `jqt --config /path/to/custom.toml`
- **THEN** the loader reads from the specified path instead of the XDG default

### Requirement: Merge user config over compiled-in defaults
The loader SHALL apply a partial-override strategy: any action not present in the config file retains its compiled-in default binding. The user need not specify every action.

#### Scenario: Partial config file
- **WHEN** the config file contains only `[keys] quit = "F10"` and no other entries
- **THEN** `quit` is bound to F10 and all other actions retain their default bindings

#### Scenario: Empty config file
- **WHEN** the config file exists but is empty
- **THEN** all actions use their compiled-in defaults

### Requirement: Report parse errors in footer without aborting
When the config file exists but contains invalid TOML or an unrecognised action name or an unparseable key string, the loader SHALL log the error to the footer at startup and continue with compiled-in defaults.

#### Scenario: Invalid TOML syntax
- **WHEN** the config file contains malformed TOML (e.g. missing quotes)
- **THEN** the footer shows a one-line config error message and all defaults are used

#### Scenario: Unknown action name
- **WHEN** the config file references an action name that does not exist in the registry (e.g. `[keys] frobnicate = "Ctrl+X"`)
- **THEN** the unknown entry is skipped, a warning is added to the footer message, and known entries are applied normally

#### Scenario: Unparseable key string
- **WHEN** a key string cannot be parsed (e.g. `"Ctrl+"` with no key, or `"Hyperspace+Z"`)
- **THEN** that binding is skipped with a warning; other valid entries are applied

### Requirement: Detect and reject conflicting bindings
The loader SHALL detect when two different actions are mapped to the same key chord in the final resolved keymap. On conflict, the entire user config is rejected and defaults are used.

#### Scenario: Two actions bound to the same key
- **WHEN** the config maps both `copy_clipboard` and `save_output` to `"Ctrl+S"`
- **THEN** the loader rejects the config, shows a conflict error in the footer, and uses compiled-in defaults for all actions

### Requirement: --print-config flag outputs effective config and exits
When `jqt --print-config` is invoked, the application SHALL print the full effective TOML configuration (compiled-in defaults merged with any user overrides) to stdout and exit with code 0.

#### Scenario: Default config printed
- **WHEN** the user runs `jqt --print-config` with no config file
- **THEN** a valid TOML document listing all actions with their default bindings is written to stdout

#### Scenario: Merged config printed
- **WHEN** the user runs `jqt --print-config` with a partial config file
- **THEN** the printed TOML reflects the merged result (user overrides shown, defaults for the rest)
