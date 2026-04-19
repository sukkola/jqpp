# jq++

**jq++** (binary: `jqpp`) is a robust TUI for interactively exploring JSON with jq-like queries, powered by [`jaq`](https://github.com/01mf02/jaq).

<img src="demo/demo.gif" alt="jqpp demo">

## Why jq++?

While tools like [`jqp`](https://github.com/noahgorstein/jqp) provide a great interactive TUI for jq, they lack intelligent assistance during query authoring. **jq++** bridges this gap by adding deep intellisense features that make authoring complex jq filters significantly faster and more accurate:

- **JSON-context field completions**: jq++ walks your live JSON input (up to 4 levels deep) to suggest actual field names and array indices at your current cursor position.
- **Type-aware builtin catalog**: Suggestions for jq's ~90 built-in functions are filtered by the runtime type flowing into the pipe. For example, `ascii_upcase` is only suggested after a string-producing expression.
- **LSP Integration**: Automatic integration with [`jq-lsp`](https://github.com/wader/jq-lsp) if it is found on your PATH — provides real-time diagnostics and function signatures directly in the TUI footer.

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
brew tap sukkola/tap
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

# Disable LSP even if jq-lsp is on PATH
jqpp data.json --no-lsp

# Print selected content to stdout on exit
jqpp data.json --print-output
jqpp data.json --print-query
jqpp data.json --print-input
```

### Pipeline usage

```bash
# Explore interactively, then pipe final result
jqpp data.json --print-output | jq -r '.[0].name'

# Reuse the final query in another command
jqpp data.json --print-query | xargs -I{} jq '{}' data.json

# Pass the original input through another tool after interactive editing
jqpp data.json --print-input | jq 'keys'
```

### Keybindings

| Key | Action |
|---|---|
| `Enter` | Accept highlighted completion, or execute query |
| `Tab` | Accept completion, or cycle focus to next pane |
| `Shift+Tab` | Cycle focus to previous pane |
| `Down` | Move completion selection down; open dropdown from cache |
| `Up` | Move completion selection up; navigate query history |
| `Page Down` | Scroll focused input/output pane down by one viewport |
| `Page Up` | Scroll focused input/output pane up by one viewport |
| `Home` | Jump focused input/output pane to top |
| `End` | Jump focused input/output pane to bottom |
| `Esc` | Dismiss completion dropdown |
| `Esc Esc` | Clear the query bar (double-press within 500ms) |
| `Ctrl+T` | Toggle query bar visibility |
| `Ctrl+M` | Toggle side menu |
| `Ctrl+Y` | Copy focused pane to clipboard |
| `Ctrl+S` | Save output to `jqpp-output.json` |
| `q` | Quit (when focus is not on query input) |
| `Ctrl+C` | Quit from any state |

Input and output panes also support scrollbar interaction with the mouse: use wheel/trackpad over the pane under the pointer, or click and drag directly on the pane scrollbar.

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

jq++ automatically detects and uses [`jq-lsp`](https://github.com/wader/jq-lsp) if it is present on your PATH. Install it with:

```bash
go install github.com/wader/jq-lsp@latest
```

Once installed, jq-lsp starts automatically when you launch jqpp — no flags needed. It adds real-time parse diagnostics shown in the footer.

To disable LSP for a session, pass `--no-lsp`. To use a binary at a custom path, set `JQPP_LSP_BIN`:

```bash
JQPP_LSP_BIN=/usr/local/bin/jq-lsp jqpp data.json
```

## Related Projects

- [jaq](https://github.com/01mf02/jaq) — The Rust jq implementation powering jq++.
- [jqp](https://github.com/noahgorstein/jqp) — The original Go TUI inspiration for this project.
- [jq-lsp](https://github.com/wader/jq-lsp) — The language server providing diagnostics.

## Building

```bash
cargo build --release
```

Requires Rust 1.90.0 or later.
