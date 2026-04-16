## 1. Cargo Package Identity

- [x] 1.1 In `Cargo.toml`, change `name = "jqt"` to `name = "jqpp"` and update `description`
- [x] 1.2 In `Cargo.toml`, add or update `[[bin]]` section so `name = "jqpp"` and `path = "src/main.rs"`

## 2. Source Code Renames

- [x] 2.1 In `src/completions/lsp.rs`, replace `JQT_LSP_BIN` env var lookup with `JQPP_LSP_BIN`
- [x] 2.2 In `src/config.rs`, change config directory path from `jqt` to `jqpp`
- [x] 2.3 In `src/config.rs`, add fallback: if `~/.config/jqpp/` missing and `~/.config/jqt/` exists, read from old path and set a one-time footer notice string (DEPRECATED per user instruction)
- [x] 2.4 Search all source files for remaining `jqt` string literals in user-visible text (help strings, error messages, comments) and update to `jqpp` / `jq++`

## 3. Documentation

- [x] 3.1 Update `README.md`: rename tool references, installation instructions, binary name, env var name, config path
- [x] 3.2 Update any other docs/markdown files in the repo that reference `jqt`

## 4. Verification

- [x] 4.1 Run `cargo build` and confirm the produced binary is named `jqpp`
- [x] 4.2 Confirm `JQPP_LSP_BIN` is respected (manual test or existing LSP integration test)
- [x] 4.3 Confirm config loads from `~/.config/jqpp/` on a clean install path

