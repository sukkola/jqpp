## ADDED Requirements

### Requirement: Optional LSP activation via CLI flag
The LSP provider SHALL only start when the `--lsp` flag is passed at launch.

#### Scenario: LSP disabled by default
- **WHEN** `jqt` is launched without `--lsp`
- **THEN** no `jq-lsp` subprocess is spawned and no LSP completions appear

#### Scenario: LSP enabled with flag
- **WHEN** `jqt` is launched with `--lsp`
- **THEN** a `jq-lsp` subprocess is spawned over stdio and the LSP handshake begins

### Requirement: LSP initialize handshake
After spawning, the provider SHALL send an LSP `initialize` request followed by `initialized` notification and wait for `InitializeResult` before issuing other requests.

#### Scenario: Successful handshake
- **WHEN** `jq-lsp` responds to `initialize`
- **THEN** the provider transitions to `ready` state; the "LSP initializing…" footer indicator is hidden

#### Scenario: Handshake timeout
- **WHEN** `jq-lsp` does not respond within 5 seconds
- **THEN** the provider disables itself and shows "LSP unavailable" in the footer for 3 seconds

### Requirement: Completions via textDocument/completion
For each debounced query edit, the provider SHALL send `textDocument/didChange` then `textDocument/completion` and deliver the resulting items.

#### Scenario: Completion items returned
- **WHEN** the user types `env` and LSP is ready
- **THEN** the provider delivers items such as `env`, `$ENV` from jq-lsp

#### Scenario: LSP cache only updated on non-empty response
- **WHEN** jq-lsp returns an empty completion list (e.g. for the `as` keyword)
- **THEN** the previous LSP cache is retained; the suggestion dropdown is not cleared

### Requirement: LSP completions ranked third
LSP items appear after json_context and jq_builtins in the merged suggestion list. They are filtered client-side by the current token prefix before merging so stale LSP cache does not cause flickering.

#### Scenario: Stale cache filtered by token prefix
- **WHEN** the cached LSP results contain items that do not start with the current token
- **THEN** those items are excluded from the rendered suggestion list

#### Scenario: LSP cache cleared on suggestion accept
- **WHEN** the user accepts a suggestion (Tab or Enter)
- **THEN** `lsp_completions` and `cached_pipe_type` are cleared; fresh completions are computed on the next edit

### Requirement: Diagnostics displayed in footer
The provider SHALL display the first LSP diagnostic message in the footer. When there are no diagnostics the region is empty.

#### Scenario: Error diagnostic shown
- **WHEN** jq-lsp publishes a `textDocument/publishDiagnostics` notification with severity Error
- **THEN** the error message is shown in the footer in red

#### Scenario: Diagnostics cleared on valid query
- **WHEN** jq-lsp publishes an empty diagnostics array
- **THEN** the footer diagnostic region is cleared

### Requirement: LSP subprocess cleanup on exit
When the application exits, the provider SHALL send `shutdown` + `exit` to the subprocess.

#### Scenario: Clean shutdown
- **WHEN** the user quits `jqt`
- **THEN** `shutdown` + `exit` are sent to `jq-lsp` and the child process terminates cleanly
