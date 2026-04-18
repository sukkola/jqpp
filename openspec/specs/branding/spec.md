# branding Specification

## Purpose
TBD - created by archiving change rename-to-jqpp. Update Purpose after archive.
## Requirements
### Requirement: Binary is named jqpp
The installed binary SHALL be named `jqpp`. Users invoke the tool as `jqpp <args>`.

#### Scenario: Binary invocation
- **WHEN** a user runs `jqpp` in their terminal
- **THEN** the tool launches normally (same behavior as the old `jqt` binary)

### Requirement: Cargo package is named jqpp
The Cargo package name and binary target SHALL both be `jqpp` so `cargo install` installs a binary named `jqpp`.

#### Scenario: Cargo install
- **WHEN** a user runs `cargo install jqpp`
- **THEN** a binary named `jqpp` is placed in `~/.cargo/bin/`

### Requirement: Config directory is jqpp
The application SHALL read configuration from `$XDG_CONFIG_HOME/jqpp/` (defaulting to `~/.config/jqpp/`).

#### Scenario: Fresh install config location
- **WHEN** no config directory exists
- **THEN** the app uses `~/.config/jqpp/config.toml` for config

### Requirement: LSP binary env var is JQPP_LSP_BIN
The environment variable for overriding the LSP binary path SHALL be `JQPP_LSP_BIN`.

#### Scenario: Custom LSP binary
- **WHEN** `JQPP_LSP_BIN=/usr/local/bin/my-lsp` is set
- **THEN** the app spawns that binary as the LSP process

#### Scenario: Old env var is ignored
- **WHEN** only `JQT_LSP_BIN` is set (not `JQPP_LSP_BIN`)
- **THEN** the app falls back to the default `jq-lsp` binary name (old var is not read)

