## Why

`jqp` is a popular Go TUI for interactively exploring JSON with jq, but it lacks intelligent query assistance. This project reimplements and extends it in Rust/ratatui, adding dual-source intellisense (JSON-context field suggestions + LSP-backed jq function completions) and a future-ready side-menu layout, producing a faster, more productive jq explorer called `jqt`.

## What Changes

- New Rust/ratatui binary `jqt` replacing the Go `jqp` baseline
- jq execution powered by [`jaq`](https://github.com/01mf02/jaq) (Rust-native, no external `jq` binary required)
- Input field with ghost-text completions fed by two independent providers:
  - **JSON-context provider**: walks the live query result to surface field names and values reachable from the current cursor position
  - **LSP provider**: optionally connects to [`jq-lsp`](https://github.com/wader/jq-lsp) via JSON-RPC for function signatures, built-in documentation, and error diagnostics
- jqp-compatible keyboard layout (Enter, Tab, Ctrl+Y, Ctrl+T, Ctrl+S)
- Extendable side-menu panel (hidden by default; layout slot reserved for Query / Config / Runs / Output / History / Saved)
- Two-pane layout (input JSON left, query output right) matching jqp's visual design

## Capabilities

### New Capabilities

- `tui-layout`: Main ratatui application shell — top query input, left JSON pane, right output pane, footer keybindings bar, optional side-menu column
- `query-input`: Single-line query input widget with history, ghost-text inline completions, and cursor tracking
- `json-context-completions`: Analyzes current jq query + input JSON to suggest reachable field names and values
- `lsp-completions`: Optional jq-lsp integration over stdio JSON-RPC providing function completions and diagnostics
- `jaq-executor`: Runs jq queries via the `jaq` library crate, streams results back to the output pane
- `side-menu`: Collapsible left-side navigation menu (initially stub items, toggled by a keybinding)

### Modified Capabilities

## Impact

- New Rust project (`Cargo.toml`) in this repo; no existing source to modify
- Runtime dependency: `jq-lsp` binary optional (enabled via `--lsp` CLI flag)
- No external `jq` binary needed (jaq is a library dependency)
- Existing Go attempt at `/Users/sampo/Sources/photo-evaluator/jqadditions/jqp` is reference only — not modified
