## ADDED Requirements

### Requirement: Execute jq query via jaq library
The executor SHALL evaluate a jq query string against a JSON input value using `jaq-core` + `jaq-std`. No external `jq` binary is required.

#### Scenario: Valid query returns output
- **WHEN** the query is `.name` and the input JSON is `{"name":"alice"}`
- **THEN** the output pane shows `"alice"`

#### Scenario: Query returning multiple values
- **WHEN** the query is `.[]` and the input is `[1,2,3]`
- **THEN** each value is rendered on a separate line in the output pane

#### Scenario: Multi-stage pipe query
- **WHEN** the query is `.config.name | ascii_upcase`
- **THEN** the result reflects the full pipe chain

### Requirement: Result cap
The executor SHALL cap output at 10 000 results to prevent unbounded memory use on large inputs.

#### Scenario: Large output truncated
- **WHEN** a query produces more than 10 000 values
- **THEN** only the first 10 000 are stored and displayed

### Requirement: Execute on Enter
The executor SHALL run the query when the user presses Enter (without a dropdown active) and update the output pane.

#### Scenario: Output pane updated
- **WHEN** user presses Enter with a valid query
- **THEN** the output pane content is replaced within one render frame

### Requirement: Live debounced execution
The executor SHALL also run the query automatically after each debounce tick (80 ms idle) and update the output pane without requiring Enter.

#### Scenario: Live update on edit
- **WHEN** the user pauses typing for 80 ms
- **THEN** the output pane refreshes with the current query result

### Requirement: Display execution errors in output pane
When jaq returns a compile or runtime error, the executor SHALL display the error in the output pane in red.

#### Scenario: Compile error displayed
- **WHEN** the query is syntactically invalid
- **THEN** the output pane shows the jaq parse error in red

#### Scenario: Runtime error displayed
- **WHEN** the query is valid but fails at runtime
- **THEN** the output pane shows the runtime error in red

### Requirement: Accept input from stdin or file
The executor SHALL read input JSON from stdin when no file argument is given, or from the file path provided as the first positional CLI argument.

#### Scenario: Stdin input
- **WHEN** `jqt` is launched as `echo '{}' | jqt`
- **THEN** `{}` is used as the input JSON

#### Scenario: File input
- **WHEN** `jqt` is launched as `jqt data.json`
- **THEN** the contents of `data.json` are used as the input JSON

### Requirement: Show input size in left pane status bar
The left pane SHALL display the input source label and byte size (e.g. `stdin | 2.4 KB`).

#### Scenario: Status bar label
- **WHEN** input is read from stdin
- **THEN** the left pane status bar shows `stdin | <size>`

### Requirement: Large input display truncation
When raw input exceeds 64 KB the left pane SHALL display only the first 64 KB followed by a truncation notice rather than freezing the render loop.

#### Scenario: Truncation notice
- **WHEN** the raw input is larger than 64 KB
- **THEN** the display string ends with `[… N KB total, display truncated]`

### Requirement: Format operator interception (@csv, @tsv)
Because `jaq` does not natively support `@csv` / `@tsv`, the executor SHALL intercept queries ending with `| @csv` or `| @tsv`, evaluate the base expression via jaq, and apply a Rust-implemented CSV/TSV formatter.

#### Scenario: @csv output
- **WHEN** the query ends with `| @csv` and the base expression produces arrays
- **THEN** each array is rendered as a quoted, comma-separated row

#### Scenario: @tsv output
- **WHEN** the query ends with `| @tsv` and the base expression produces arrays
- **THEN** each array is rendered as a tab-separated row

#### Scenario: @csv/@tsv sets raw output mode
- **WHEN** a format operator query executes successfully
- **THEN** string results are displayed without surrounding JSON quotes (`raw_output = true`)

#### Scenario: Ctrl+Y copies format operator output
- **WHEN** the user presses Ctrl+Y with the right pane focused
- **THEN** the raw (unquoted) text is copied, regardless of whether `app.error` is set

### Requirement: Non-blocking query execution
The executor SHALL run jaq queries in a background OS thread (`spawn_blocking`) and store the result `JoinHandle` rather than awaiting it inline. The main event loop polls the handle each frame so that:
- Ctrl+C and all keyboard input remain responsive during long-running queries.
- A new debounce tick discards the in-flight handle and spawns a fresh one with the latest query.

#### Scenario: Long query does not freeze input
- **WHEN** a query takes several seconds to evaluate on a large input
- **THEN** the user can still press Ctrl+C or type new characters without the UI hanging

#### Scenario: New query supersedes in-flight compute
- **WHEN** the user types a new query before the previous computation has finished
- **THEN** the previous JoinHandle is dropped and a new computation starts with the latest query

### Requirement: Improved error message formatting
The executor SHALL format jaq load and compile errors to include a short, human-readable description of the problematic token.

#### Scenario: Lex error shows problem token
- **WHEN** jaq reports a `Lex` error (e.g. single-quoted string instead of double-quoted)
- **THEN** the error message shows the first 30 characters of the offending token so the user can identify the issue

#### Scenario: Undefined name error
- **WHEN** jaq reports an undefined name (e.g. `@csv` typed without the interception path)
- **THEN** the error message shows `undefined: <name>` for each unknown identifier
