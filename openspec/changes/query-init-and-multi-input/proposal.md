## Why

When embedding jqpp in scripts or editor integrations, callers need to pre-populate the query and cursor position so the tool opens ready-to-use rather than requiring the user to type from scratch. Similarly, exploring several JSON files together as a unified dataset is a common need that currently requires manual array construction outside the tool.

## What Changes

- New `--query <expr>` CLI flag sets the initial query string in the query bar on startup; suggestions fire immediately if the query is non-trivial.
- New `--cursor <col>` CLI flag sets the initial cursor column within the pre-filled query (0-based character offset), enabling callers to place the cursor inside a function argument so completions appear at exactly the right point on first keypress.
- New multi-file / exploratory mode: when more than one positional file argument is given (or one or more files combined with stdin pipe input), all inputs are merged into a JSON array of their parsed values and loaded as a single dataset.
- In multi-file mode, the default query is `.[]` (unless `--query` is also given), since the outer array wrapper is an implementation detail the user would want to unwrap immediately.
- A positional file list is now `Vec<PathBuf>` instead of `Option<PathBuf>`; the existing single-file behaviour is unchanged.

## Capabilities

### New Capabilities

- `initial-query`: CLI parameters for pre-filling the query bar and cursor position on startup, including immediate suggestion activation.
- `multi-file-input`: Accepting multiple input files (and mixing files with stdin pipe), merging them into a single JSON array dataset with `.[]` as the automatic default query.

### Modified Capabilities

- `cli-output-selection`: The `file` positional argument changes from `Option<PathBuf>` to `Vec<PathBuf>`. Existing `--print-*` flag semantics are unchanged, but the spec should reflect the updated positional argument multiplicity.

## Impact

- `src/main.rs` (Args struct, `actual_main`, `run`): new fields `query`, `cursor`, multiple `file` positionals.
- `src/app.rs` (`App::new`, query field init): initial query and cursor applied before first draw.
- Integration tests (`tests/pipe_integration.rs`, new CLI tests): cover `--query`, `--cursor`, and multi-file scenarios.
- No library-crate public API changes; all changes are in the binary crate.
