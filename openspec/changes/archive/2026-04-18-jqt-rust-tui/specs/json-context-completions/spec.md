## ADDED Requirements

### Requirement: Extract field names from query result
The JSON-context provider SHALL walk the current jaq query result (or the raw input JSON when the query is empty or invalid) and collect all reachable object key names up to depth 4.

#### Scenario: Top-level keys suggested
- **WHEN** the input JSON is `{"name":"alice","age":30}` and the query is `.`
- **THEN** the provider returns completions `.name` and `.age`

#### Scenario: Nested keys suggested with path prefix
- **WHEN** the input JSON contains `{"user":{"id":1,"email":"a@b.com"}}` and the query is `.user`
- **THEN** the provider returns `.id` and `.email` relative to the cursor context

#### Scenario: Depth cap respected
- **WHEN** the JSON tree is deeper than 4 levels
- **THEN** keys beyond depth 4 are not included in suggestions

### Requirement: Filter suggestions by current input prefix
The provider SHALL only return completions whose label starts with the text after the last `.` or `|` in the current query (case-sensitive).

#### Scenario: Prefix filter applied
- **WHEN** the query is `.na` and top-level keys are `name`, `age`
- **THEN** only `.name` is returned

#### Scenario: No prefix returns all keys
- **WHEN** the query ends with `.`
- **THEN** all keys at the current context level are returned

### Requirement: Debounce completion requests
The provider SHALL wait at least 80 ms of idle input before computing new suggestions.

#### Scenario: Rapid typing does not thrash
- **WHEN** the user types multiple characters within 80 ms
- **THEN** the provider computes suggestions only once after the idle period elapses

### Requirement: Graceful handling of invalid queries
When the current query is invalid jq, the provider SHALL fall back to walking the raw input JSON from the root.

#### Scenario: Invalid query fallback
- **WHEN** the query is syntactically invalid
- **THEN** the provider returns top-level key completions from the raw input JSON

### Requirement: Three-source merged completion pipeline
Completions are assembled from three sources in priority order. First-seen deduplication by label is applied across all three.

1. **JSON-context** — object field paths derived from the live query result
2. **jq builtins** — type-filtered builtin catalog (see jq-builtins spec)
3. **LSP** — stale-cache results from jq-lsp, filtered by the current token prefix

#### Scenario: JSON-context items rank first
- **WHEN** the same label appears in both json_context and builtins/LSP
- **THEN** the json_context item is kept and the duplicate is dropped

#### Scenario: Builtins rank second
- **WHEN** a label appears in builtins and LSP but not json_context
- **THEN** the builtin item is kept

### Requirement: Pipe-prefix type detection
After each debounce tick, the executor evaluates the expression to the left of the last `|` and records the runtime JSON type of its output as `cached_pipe_type`.

#### Scenario: Type detected after pipe
- **WHEN** the query is `.name | ascii_up` and `.name` produces a string
- **THEN** `cached_pipe_type` is `"string"` and only string-compatible builtins are shown

#### Scenario: No pipe — type is unknown
- **WHEN** the query contains no `|`
- **THEN** `cached_pipe_type` is `None` and all builtins are shown
