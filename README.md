# jq++

**jq++** (binary: `jqpp`) is a robust TUI for interactively exploring JSON with jq-like queries, powered by [`jaq`](https://github.com/01mf02/jaq).

<img src="demo/demo.gif" alt="jqpp demo">

## Why jq++?

While tools like [`jqp`](https://github.com/noahgorstein/jqp) provide a great interactive TUI for jq, they lack intelligent assistance during query authoring. **jq++** bridges this gap by adding deep intellisense features that make authoring complex jq filters significantly faster and more accurate:

- **JSON-context field completions**: jq++ walks your live JSON input (up to 4 levels deep) to suggest actual field names and array indices at your current cursor position.
- **Type-aware builtin catalog**: Suggestions for jq's ~90 built-in functions are filtered by the runtime type flowing into the pipe. For example, `ascii_upcase` is only suggested after a string-producing expression.
- **LSP Integration**: Optional integration with [`jq-lsp`](https://github.com/wader/jq-lsp) provides real-time diagnostics and function signatures directly in the TUI footer.

## Features

- **Live evaluation** — results update as you type (80ms debounce).
- **Dual-pane layout** — input JSON on the left, query output on the right.
- **Mid-query editing** — completions work at any cursor position, not just at the end.
- **Format operators** — native support for `@csv` and `@tsv` (which `jaq` lacks).
- **Non-blocking execution** — heavy queries run in the background; the UI stays responsive.
- **Query history** — Navigate previous queries with Up/Down.
- **Clipboard copy** — `Ctrl+Y` copies the focused pane's content.

## Installation

### Homebrew (macOS/Linux)

```bash
brew install sukkola/tap/jqpp
```

### Cargo

```bash
cargo install jqpp
```

## Usage

```bash
# Read from a file
jqpp data.json

# Read from stdin
cat data.json | jqpp

# Enable jq-lsp integration
jqpp data.json --lsp
```

### Keybindings

| Key | Action |
|---|---|
| `Enter` | Accept highlighted completion, or execute query |
| `Tab` | Accept completion, or cycle focus to next pane |
| `Shift+Tab` | Cycle focus to previous pane |
| `Down` | Move completion selection down; open dropdown from cache |
| `Up` | Move completion selection up; navigate query history |
| `Esc` | Dismiss completion dropdown |
| `Esc Esc` | Clear the query bar (double-press within 500ms) |
| `Ctrl+T` | Toggle query bar visibility |
| `Ctrl+M` | Toggle side menu |
| `Ctrl+Y` | Copy focused pane to clipboard |
| `Ctrl+S` | Save output to `jqpp-output.json` |
| `q` | Quit (when focus is not on query input) |
| `Ctrl+C` | Quit from any state |

## Configuration

`jqpp` looks for a TOML config file at `~/.config/jqpp/config.toml` (or `$XDG_CONFIG_HOME/jqpp/config.toml`).

```toml
[keys]
quit = "F10"
copy-clipboard = "Ctrl+C"
save-output = "Ctrl+S"
next-pane = "Ctrl+Right"
prev-pane = "Ctrl+Left"
```

Use `jqpp --print-config` to see the full list of remappable actions and their current bindings.

## jq-lsp Setup

For diagnostics and additional function signatures, install `jq-lsp`:

```bash
go install github.com/wader/jq-lsp@latest
```

Enable it in jq++ with the `--lsp` flag. You can override the binary path using the `JQPP_LSP_BIN` environment variable.

## Related Projects

- [jaq](https://github.com/01mf02/jaq) — The Rust jq implementation powering jq++.
- [jqp](https://github.com/noahgorstein/jqp) — The original Go TUI inspiration for this project.
- [jq-lsp](https://github.com/wader/jq-lsp) — The language server providing diagnostics.

## Building

```bash
cargo build --release
```

Requires Rust 1.90.0 or later.
