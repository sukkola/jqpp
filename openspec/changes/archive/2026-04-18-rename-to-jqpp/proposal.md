## Why

A CLI tool named `jqt` already exists on crates.io and npm for jq templating, creating a naming conflict and discoverability confusion. Renaming to `jqpp` (long form: jq++) clearly signals the tool's purpose — an enhanced, interactive jq experience — while avoiding collision with the existing `jqt` tool.

## What Changes

- **BREAKING** Binary name changes from `jqt` to `jqpp`
- Cargo package name changes from `jqt` to `jqpp`
- All user-facing strings, help text, and error messages updated from `jqt`/`jqt` to `jqpp`/`jq++`
- Config file location moves from `~/.config/jqt/` to `~/.config/jqpp/`
- Environment variable prefix changes from `JQT_` to `JQPP_` (e.g. `JQPP_LSP_BIN`)
- Repository name and documentation updated

## Capabilities

### New Capabilities

- `branding`: Name, binary, package identity, and config path all reflect `jqpp`/`jq++`

### Modified Capabilities

<!-- No existing spec-level behavioral requirements are changing — this is a pure rename/rebrand. -->

## Impact

- `Cargo.toml`: `name`, `description`, binary target
- `src/main.rs`: CLI struct name/doc strings, `JQT_LSP_BIN` env var reference
- `src/config.rs`: config directory path (`jqt` → `jqpp`)
- `README.md`, any docs referencing the old name
- Users with existing config at `~/.config/jqt/` will need to migrate (or the app can check the old path as a fallback)
