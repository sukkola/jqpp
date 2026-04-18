## 1. Args & Output Mode

- [x] 1.1 Add `OutputMode` enum (`Result`, `Query`, `Input`) to `src/main.rs`
- [x] 1.2 Add three `bool` fields to `Args`: `print_result`, `print_query`, `print_input`, each with `#[arg(long)]`
- [x] 1.3 Add a clap `ArgGroup` named `"output"` to `Args` that makes the three flags mutually exclusive (at most one allowed); set `conflicts_with` or `group` appropriately so clap generates a clear error automatically
- [x] 1.4 Add a helper function or inline logic in `actual_main` to convert the three bools to `Option<OutputMode>` after parsing

## 2. Thread Output Mode Through run()

- [x] 2.1 Change `run()` return type from `Result<()>` to `Result<App>` so the caller can access final app state
- [x] 2.2 Update the single call site in `actual_main` to unwrap the returned `App`
- [x] 2.3 Pass `Option<OutputMode>` into `run()` (or store it on `App`) so the exit path knows what to print

## 3. Print on Exit

- [x] 3.1 After TUI teardown in `actual_main`, match on `OutputMode`:
  - `Result` → call `Executor::format_results(&app.results, app.raw_output)` and print to stdout with a trailing newline
  - `Query` → print `app.query_input.value()` with a trailing newline
  - `Input` → print the raw JSON string from `app.executor` with a trailing newline
- [x] 3.2 If `app.results` is empty when `OutputMode::Result` is active, print nothing (empty output, not an error)

## 4. Documentation

- [x] 4.1 Add `--print-result`, `--print-query`, `--print-input` to the usage section in `README.md`
- [x] 4.2 Add a "Pipeline usage" subsection to README with at least one example per flag (e.g. `jqpp data.json --print-result | jq -r '.[0]'`)

## 5. Tests

- [x] 5.1 Add a test in `tests/pipe_integration.rs` (or a new `tests/output_flag_tests.rs`) that spawns `jqpp` with a small JSON file and `--print-result`, sends a simple query via stdin simulation or `JQPP_SKIP_TTY_CHECK`, and asserts the expected result appears on stdout
- [x] 5.2 Add a test for `--print-query` that asserts the query string is echoed to stdout on exit
- [x] 5.3 Add a test that passes both `--print-result` and `--print-query` and asserts the process exits with a non-zero status without writing any TUI output
