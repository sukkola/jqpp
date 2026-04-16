# jqt

`jqt` is a Rust TUI for interactively exploring JSON with jq-like queries, powered by [`jaq`](https://github.com/01mf02/jaq). It is inspired by `jqp` but written in Rust with additional features: type-aware completions, format-operator support, and a non-blocking event loop that stays responsive on large inputs.

## Features

- **Live query evaluation** — results update as you type (80 ms debounce)
- **Dual-pane layout** — input JSON on the left, query output on the right
- **Type-aware completions** — dropdown and ghost-text suggestions from three merged sources:
  - **JSON-context** — field names and paths derived from the live input
  - **jq builtins catalog** — ~90 built-in functions filtered by the runtime type flowing into `|`
  - **LSP** (optional) — function signatures and diagnostics from `jq-lsp`
- **Format operators** — `@csv` and `@tsv` work even though `jaq` doesn't support them natively
- **Mid-query editing** — completions are context-correct at any cursor position, not just the end
- **Paste performance** — bracketed paste mode prevents per-character intellisense thrash
- **Non-blocking execution** — heavy queries run in a background thread; Ctrl+C always works
- **Query history** — Up/Down navigates previous queries
- **Clipboard copy** — Ctrl+Y copies the focused pane's content (query, raw input, or output)

## Usage

```bash
# Read from a file
jqt data.json

# Read from stdin
cat data.json | jqt

# Enable jq-lsp integration (requires jq-lsp on PATH)
jqt data.json --lsp
```

## Keybindings

| Key | Action |
|---|---|
| `Enter` | Accept highlighted completion, or execute query |
| `Tab` | Accept completion, or cycle focus to next pane |
| `Shift+Tab` | Cycle focus to previous pane |
| `Down` | Move completion selection down; open dropdown from cache |
| `Up` | Move completion selection up; navigate query history |
| `Esc` | Dismiss completion dropdown |
| `Esc Esc` | Clear the query bar (double-press within 500 ms) |
| `Ctrl+T` | Toggle query bar visibility |
| `Ctrl+M` | Toggle side menu |
| `Ctrl+Y` | Copy focused pane to clipboard (query / raw input / output) |
| `Ctrl+S` | Save output to `jqt-output.json` |
| `q` | Quit (when focus is not on query input) |
| `Ctrl+C` | Quit from any state |

## Completions

Suggestions appear after typing `.`, `|`, `{`, `[`, `,`, or `@`. They also update as you continue typing a prefix. The dropdown shows up to 11 items at a time and scrolls as you navigate.

**Type-aware filtering**: when your query has a pipe (`|`), `jqt` evaluates the expression before the pipe to determine its runtime type. Only functions compatible with that type are suggested — for example, `ascii_upcase` only appears after a string-producing expression.

**Mid-query editing**: if you move your cursor into the middle of an existing query and type, completions are based on the text to the left of the cursor. Accepting a completion preserves whatever was to the right.

## Format Operators

`jqt` supports `@csv` and `@tsv` even though `jaq` does not implement them natively. The operators are intercepted at the executor level:

```
# Produces comma-separated rows
.rows[] | @csv

# Produces tab-separated rows
.rows[] | @tsv
```

Other format operators (`@base64`, `@base64d`, `@uri`, `@html`, `@sh`, `@json`, `@text`) pass through to `jaq` directly.

## LSP Support

Pass `--lsp` to enable [`jq-lsp`](https://github.com/wader/jq-lsp) integration. `jq-lsp` must be installed and on your `PATH`.

When active, the footer shows the LSP status. Function signatures and additional completions from `jq-lsp` are merged into the dropdown behind the built-in catalog. Parse errors from `jq-lsp` appear in the footer in red.

## Known `jaq` Compatibility Gaps

`jqt` uses `jaq` as its engine. Known unsupported features:

- `$ENV` — environment variable access
- `path()` expressions
- Some advanced `@format` string operators (other than `@csv`/`@tsv` which are handled natively)

Unsupported features produce a clear error in the output pane.

## Building

```bash
cargo build --release
```

Requires Rust 1.90.0 or later (edition 2024). No external `jq` binary is needed — `jaq` is compiled in.
