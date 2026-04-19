## MODIFIED Requirements

### Requirement: Multiple file arguments are merged into a JSON array
The system SHALL accept two or more positional file arguments and merge their parsed values (JSON or YAML) into a single `serde_json::Value::Array` that is used as the dataset for the session.

#### Scenario: Two JSON object files merged into array
- **WHEN** jqpp is launched with `a.json` and `b.json`, each containing a JSON object
- **THEN** the input pane shows a JSON array with two elements (the objects from each file)

#### Scenario: Mixed JSON and YAML files merged into array
- **WHEN** jqpp is launched with `data.json` and `config.yaml`
- **THEN** the merged array contains the parsed JSON object and the parsed YAML value as elements

#### Scenario: Two YAML files merged into array
- **WHEN** jqpp is launched with `a.yaml` and `b.yml`
- **THEN** the merged array contains the JSON-equivalent values of both YAML documents

#### Scenario: Mixed types are preserved in the array
- **WHEN** jqpp is launched with three files containing a JSON object, a JSON array, and a JSON string respectively
- **THEN** the merged input is `[ <object>, <array>, <string> ]` with all three values present

#### Scenario: Single file still behaves as today
- **WHEN** jqpp is launched with exactly one positional file
- **THEN** the input is used as-is (not wrapped in an array), identical to current behaviour

#### Scenario: Non-parseable file is loaded as a JSON string
- **WHEN** one of the multiple input files has no recognised extension and contains non-JSON text
- **THEN** that file's content is represented as a JSON string value in the merged array

#### Scenario: Duplicate files are included as separate entries
- **WHEN** the same file path is passed twice
- **THEN** the merged array contains two copies of that file's parsed value

### Requirement: Stdin pipe combined with files is included in the merged array
The system SHALL include stdin pipe content as the first element of the merged array when stdin is piped and one or more file arguments are also given.

#### Scenario: Pipe + one file → two-element array
- **WHEN** jqpp is launched with one file and stdin pipe is active
- **THEN** the merged array is `[ <stdin value>, <file value> ]`

#### Scenario: Pipe + two files → three-element array
- **WHEN** jqpp is launched with two files and stdin pipe is active
- **THEN** the merged array is `[ <stdin value>, <file1 value>, <file2 value> ]`

#### Scenario: Pipe alone without files keeps existing single-input behaviour
- **WHEN** jqpp is launched with stdin pipe and no file arguments
- **THEN** the input is used as-is (not wrapped), identical to current behaviour

### Requirement: Multi-file mode auto-fills .[] as the default query
The system SHALL set the initial query to `.[]` when more than one input source is active and no `--query` flag was given.

#### Scenario: Default query is .[] for two files
- **WHEN** jqpp is launched with `a.json b.json` and no `--query` flag
- **THEN** the query bar shows `.[]` on the first frame

#### Scenario: Default query is .[] for a JSON file and a YAML file
- **WHEN** jqpp is launched with `a.json b.yaml` and no `--query` flag
- **THEN** the query bar shows `.[]` on the first frame

#### Scenario: Default query is .[] for file + pipe
- **WHEN** jqpp is launched with one file and a stdin pipe and no `--query` flag
- **THEN** the query bar shows `.[]` on the first frame

#### Scenario: Explicit --query overrides .[] default
- **WHEN** jqpp is launched with `a.json b.json --query '.[] | .name'`
- **THEN** the query bar shows `.[] | .name`, not `.[]`

#### Scenario: No automatic .[] for single input
- **WHEN** jqpp is launched with exactly one file or only a stdin pipe
- **THEN** the query bar is empty (or uses `--query` value if provided), not `.[]`

### Requirement: Source label reflects multiple inputs
The system SHALL show a combined source label when multiple inputs are merged.

#### Scenario: Label lists file names for multiple files
- **WHEN** jqpp is launched with `users.json` and `orders.yaml`
- **THEN** the source label shown in the UI is `users.json, orders.yaml` (or a truncated form if too long)

#### Scenario: Label includes stdin for pipe + file
- **WHEN** jqpp is launched with stdin pipe and `extra.yaml`
- **THEN** the source label is `stdin, extra.yaml`

### Requirement: File-not-found errors are reported per-file
The system SHALL report which specific file could not be read when a multi-file invocation includes a missing or unreadable path, and exit without starting the TUI.

#### Scenario: Missing file in multi-file list triggers error
- **WHEN** jqpp is launched with `exists.json nonexistent.yaml`
- **THEN** the process prints an error identifying `nonexistent.yaml` and exits with status 1 before opening the TUI
