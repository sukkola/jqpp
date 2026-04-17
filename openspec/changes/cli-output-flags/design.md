## Context

`Args` is parsed by `clap` in `main()` and passed into `actual_main(args)`. The TUI runs inside `run(terminal, app, args)`. After `run()` returns, `actual_main` restores the terminal (disables raw mode, shows cursor, leaves alternate screen). stdout is available at that point because the TUI has been torn down. The content needed for each flag:

- `--print-result`: `Executor::format_results(&app.results, app.raw_output)` — already used by Ctrl+S
- `--print-query`: `app.query_input.value()` — the current query string
- `--print-input`: the raw JSON string stored in `app.executor.as_ref().map(|e| &e.json_input)`

`app` is returned by `run()` (or we arrange it to be accessible). Currently `run()` returns `Result<()>`; it needs to either return `App` or we extract the needed content before `run()` returns.

## Goals / Non-Goals

**Goals:**
- Three mutually exclusive flags: `--print-result`, `--print-query`, `--print-input`
- Selected content written to stdout after terminal restore, terminated with a newline
- Mutual exclusion enforced at startup; error message + non-zero exit if two flags given
- Works correctly when stdin is piped (jqpp reads JSON from stdin; stdout is still the shell)
- Documented in README with pipeline examples

**Non-Goals:**
- Writing to a file (use shell redirection for that)
- Printing multiple fields at once
- Streaming output during the session
- Affecting the Ctrl+S save-to-file behaviour

## Decisions

### D1: Output enum instead of three bools

Rather than three independent `bool` fields on `Args`, add an `OutputMode` enum:

```rust
enum OutputMode { Result, Query, Input }
```

and a single `Option<OutputMode>` field on `Args` (or derive it from the three clap bools). Mutual-exclusion is then checked once in `actual_main` and the downstream code does a single `match`. Clap supports `#[arg(group = "output")]` to enforce the mutual exclusion declaratively, removing any manual check.

Alternative: three separate bools with a runtime check. Rejected — clap `ArgGroup` is the idiomatic approach and produces better error messages.

### D2: Return App from run()

Change `run()` signature from `async fn run(...) -> Result<()>` to `async fn run(...) -> Result<App>`. This lets `actual_main` access `app.results`, `app.query_input`, and `app.executor` after the TUI loop exits, without any shared state or channels.

Alternative: write the selected content to a `Option<String>` on `App` at the last moment inside `run()`. Rejected — couples output concerns to the TUI loop; returning `App` is cleaner and costs nothing.

### D3: Output format for --print-result

Use the same `Executor::format_results` path that Ctrl+S uses: newline-delimited JSON values. This is consistent and already tested.

### D4: Newline termination

Always append a trailing `\n` after the printed content so shell pipelines work naturally (`echo $(jqpp ...)` doesn't need special handling).

## Risks / Trade-offs

- [run() signature change] Any call site that ignores the return must be updated. Currently there is one call in `actual_main`. → Low risk.
- [stdin-as-JSON conflict with --print-result piping] If both stdin (JSON source) and stdout (result sink) are piped, the TUI cannot open `/dev/tty`. This is already handled by the existing `JQPP_SKIP_TTY_CHECK` path used in tests, but real users may hit it. → Mitigation: document that jqpp opens `/dev/tty` for input; the terminal must be available. No code change needed.
- [empty result on --print-result when query errors] If the query has an error at exit time, `app.results` is empty and nothing useful is printed. → Mitigation: document this behaviour; it is the correct semantics (no result = empty output).
