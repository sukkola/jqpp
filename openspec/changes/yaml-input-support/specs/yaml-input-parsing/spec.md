## ADDED Requirements

### Requirement: YAML files are parsed as YAML and converted to a JSON value
The system SHALL detect files with a `.yaml` or `.yml` extension and parse them using a YAML parser, producing a `serde_json::Value` that is used as the query input — identical to how a JSON file is used.

#### Scenario: Single YAML object file
- **WHEN** jqpp is launched with a `.yaml` file containing a YAML mapping
- **THEN** the input pane shows the JSON-equivalent object and jq queries execute against it

#### Scenario: Single YAML array file
- **WHEN** jqpp is launched with a `.yml` file containing a YAML sequence
- **THEN** the input pane shows the JSON-equivalent array

#### Scenario: YAML scalar file
- **WHEN** jqpp is launched with a `.yaml` file containing a bare scalar (e.g. `42`)
- **THEN** the input is the JSON equivalent (`42`)

#### Scenario: Malformed YAML file is a hard error
- **WHEN** jqpp is launched with a `.yaml` file that cannot be parsed as valid YAML
- **THEN** the process prints an error and exits with status 1 before opening the TUI

### Requirement: YAML stdin is detected and parsed after JSON parse fails
The system SHALL attempt to parse stdin content as YAML when JSON parsing fails, before falling back to a raw JSON string value.

#### Scenario: Valid YAML piped to stdin
- **WHEN** jqpp is launched with a valid YAML document piped to stdin (no file arguments)
- **THEN** the input is the JSON-equivalent value of that YAML document

#### Scenario: Valid JSON piped to stdin is still parsed as JSON
- **WHEN** jqpp is launched with valid JSON piped to stdin
- **THEN** the JSON path is taken (YAML attempt is not reached), and the input is correct

#### Scenario: Non-JSON non-YAML stdin still falls back to string
- **WHEN** jqpp is launched with plain text piped to stdin that is neither JSON nor YAML
- **THEN** the input is a JSON string containing the trimmed text, preserving existing fallback behaviour

### Requirement: JSON files are unaffected by YAML support
The system SHALL continue to parse `.json` files (and files with no recognised extension) using the existing JSON-first path, with no change in behaviour.

#### Scenario: JSON file with .json extension unchanged
- **WHEN** jqpp is launched with a `.json` file
- **THEN** the file is parsed as JSON, identical to behaviour before this change
