## Context

The tool is a Rust TUI binary. All user-facing identity is concentrated in a small number of places: `Cargo.toml`, one env var reference (`JQT_LSP_BIN`), the config directory path in `config.rs`, and documentation/help strings. This is a low-risk, mechanical rename with one migration concern: existing users who have config at `~/.config/jqt/`.

## Goals / Non-Goals

**Goals:**
- Binary on PATH becomes `jqpp`
- Cargo package/crate name becomes `jqpp`
- Config directory is `~/.config/jqpp/` (XDG: `$XDG_CONFIG_HOME/jqpp/`)
- Env var becomes `JQPP_LSP_BIN`
- All help text, error strings, and comments reflect the new name

**Non-Goals:**
- No behavioral changes to any feature
- No compatibility shim keeping the `jqt` binary name
- No automatic migration of user config files (document it, don't automate)

## Decisions

**Single Cargo binary target named `jqpp`**
Cargo's `[[bin]] name` field controls the installed binary name. Changing it in `Cargo.toml` is sufficient — no wrapper scripts needed.

**`JQPP_LSP_BIN` replaces `JQT_LSP_BIN`**
Env vars are part of the public interface. A clean break is better than supporting both; the tool is pre-1.0 and the change is documented.

**Config path: check old path as read-only fallback (one release only)**
On first launch after rename, if `~/.config/jqpp/` doesn't exist but `~/.config/jqt/` does, log a one-time notice in the footer and read from the old path. This softens the upgrade experience without permanent backwards-compat code. The fallback can be removed in a follow-up.

## Risks / Trade-offs

- [Users with `jqt` in shell aliases/scripts will break] → Documented in release notes; no mitigation in code
- [Config migration is manual] → Fallback read from old path covers the common case for one release
- [crates.io name `jqpp` may be taken] → Check before publishing; `jq-plus-plus` is the fallback crate name (binary still `jqpp`)
