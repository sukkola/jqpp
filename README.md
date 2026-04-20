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

## Intellisense Coverage

jqpp provides three levels of intellisense for jq/jaq built-in functions:

| Level | Meaning |
|---|---|
| **Full** | Type-aware suggestion **and** live param completions — argument values are drawn from the actual JSON input (field names, string values, array indices). |
| **Partial** | Type-aware suggestion with a static insert template. The function appears only when the right type flows into the pipe, but arguments are not auto-completed from the data. |
| **–** | No intellisense. Function is not in the catalog (unsupported by jaq, or a jq construct jqpp does not expose). |

### String functions

| Function | Level | Notes |
|---|---|---|
| `ascii_downcase` / `ascii_upcase` | Partial | |
| `ltrimstr` | **Full** | Suggests actual prefixes found in the live JSON data |
| `rtrimstr` | **Full** | Suggests actual suffixes found in the live JSON data |
| `startswith` | **Full** | Suggests actual prefixes found in the live JSON data |
| `endswith` | **Full** | Suggests actual suffixes found in the live JSON data |
| `split` | **Full** | Infers the most common delimiters from the live JSON data |
| `test` | Partial | Regex argument not completed |
| `match` | Partial | Regex argument not completed |
| `capture` | Partial | Regex argument not completed |
| `scan` | Partial | Regex argument not completed |
| `sub` / `gsub` | Partial | Regex and replacement arguments not completed |
| `explode` | Partial | |
| `fromjson` | Partial | |
| `tonumber` | Partial | |
| `strptime` | Partial | Format string not completed |
| `@base64` / `@base64d` | Partial | |
| `@uri` | Partial | |
| `@html` | Partial | |
| `@sh` | Partial | |
| `@json` / `@text` | Partial | |
| `@csv` | Partial | jqpp extension — not native to jaq |
| `@tsv` | Partial | jqpp extension — not native to jaq |

### Number functions

| Function | Level | Notes |
|---|---|---|
| `floor` / `ceil` / `round` | Partial | |
| `sqrt` / `fabs` | Partial | |
| `log` / `log2` / `log10` | Partial | |
| `exp` / `exp2` / `exp10` | Partial | |
| `pow` | Partial | |
| `isnan` / `isinfinite` / `isfinite` / `isnormal` | Partial | |
| `nan` / `infinite` | Partial | |
| `tostring` | Partial | |
| `strftime` | Partial | Format string not completed |
| `gmtime` / `mktime` | Partial | |

### Array functions

| Function | Level | Notes |
|---|---|---|
| `sort` | Partial | |
| `sort_by` | **Full** | Suggests field names from the first array element |
| `group_by` | **Full** | Suggests field names from the first array element |
| `unique` | Partial | |
| `unique_by` | **Full** | Suggests field names from the first array element |
| `flatten` | Partial | |
| `reverse` | Partial | |
| `add` | Partial | |
| `min` / `max` | Partial | |
| `min_by` / `max_by` | **Full** | Suggests field names from the first array element |
| `map` | Partial | Inner expression not completed |
| `map_values` | Partial | Inner expression not completed |
| `any` / `all` | Partial | Predicate not completed |
| `first` / `last` / `nth` | Partial | |
| `transpose` | Partial | |
| `implode` | Partial | Requires array of codepoints (`array_scalars`) |
| `from_entries` | Partial | |
| `inside` | Partial | |

### Object functions

| Function | Level | Notes |
|---|---|---|
| `to_entries` / `from_entries` / `with_entries` | Partial | |
| `keys` / `keys_unsorted` / `values` | Partial | |
| `del` | **Full** | Suggests field names/paths from the live JSON input |
| `has` | Partial | Planned: Full (live key / index completions) |

### String-or-array functions

| Function | Level | Notes |
|---|---|---|
| `contains` | Partial | Planned: Full (type-adaptive insert text + string-value completions for string input); currently shows generic template |
| `index` / `rindex` / `indices` | **Full** | Suggests actual values found at the context path in the live JSON data |
| `length` | Partial | Excluded for boolean input (`true \| length` is a jq runtime error) |
| `inside` | Partial | |

### Path and traversal functions

| Function | Level | Notes |
|---|---|---|
| `path` | **Full** | Suggests field paths from the live JSON input |
| `paths` | Partial | |
| `getpath` | Partial | |
| `setpath` / `delpaths` | Partial | |
| `recurse` | Partial | |
| `walk` | Partial | |

### Control and iteration

| Function | Level | Notes |
|---|---|---|
| `select` | Partial | Predicate not completed |
| `limit` | Partial | |
| `first(expr)` / `last(expr)` | Partial | Generator-form variants |
| `range` | Partial | |
| `reduce` | Partial | `as $var` bindings not yet offered as completions |
| `foreach` | Partial | `as $var` bindings not yet offered as completions |
| `until` / `while` | Partial | |
| `error` | Partial | |
| `empty` | Partial | |
| `debug` | Partial | jaq does not support `debug("message")` — bare `debug` only |

### Universal / misc

| Function | Level | Notes |
|---|---|---|
| `type` / `not` | Partial | |
| `tojson` / `tostring` / `tonumber` | Partial | |
| `now` / `env` | Partial | |
| `null` / `true` / `false` | Partial | Literal completions |
| `nan` / `infinite` | Partial | Literal completions |

### Not supported by jaq 3.x

These standard jq features are not available in jqpp because jaq does not implement them. They are intentionally absent from the completion catalog so jqpp never suggests something that would produce a runtime or compile error.

| Feature | Notes |
|---|---|
| `builtins` | Not implemented in jaq |
| `leaf_paths` | Not implemented in jaq; use `paths \| select(scalars)` instead |
| `ascii` (number → char) | Not implemented in jaq; use `[.] \| implode` instead |
| `recurse_down` | Alias not defined in jaq; use `recurse` |
| `input` / `inputs` | jaq has the API but jqpp does not wire it up (single-input tool) |
| `format("text")` | Not implemented in jaq; use format strings (`@base64`, `@uri`, etc.) directly |
| `$ENV` | Not available; use `env` object instead |
| `label` / `break` | Label-break control flow not in jaq |
| `?//` | Alternative operator not in jaq |
| `modulemeta` | Module system introspection not in jaq |
| `$__loc__` | Source location object not in jaq |
| `INDEX(s; f)` / `IN(s)` / `GROUP_BY(s; f)` | SQL-style operators not in jaq |
| Streaming (`truncate_stream`, `tostream`, `fromstream`) | Streaming-mode functions not in jaq |

## Related Projects

- [jaq](https://github.com/01mf02/jaq) — The Rust jq implementation powering jq++.
- [jqp](https://github.com/noahgorstein/jqp) — The original Go TUI inspiration for this project.
- [jq-lsp](https://github.com/wader/jq-lsp) — The language server providing diagnostics.

## Building

```bash
cargo build --release
```

Requires Rust 1.90.0 or later.
