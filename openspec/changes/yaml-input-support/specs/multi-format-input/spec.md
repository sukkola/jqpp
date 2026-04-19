## ADDED Requirements

### Requirement: Files are parsed by format determined from their extension
The system SHALL use `jaq_fmts::Format::determine(path)` to detect the format of each input file and parse it accordingly. Unrecognised extensions fall through to the existing JSON-first path.

#### Scenario: YAML file (.yaml) is parsed as YAML
- **WHEN** jqpp is launched with a `.yaml` file containing a valid YAML mapping
- **THEN** the input pane shows the JSON-equivalent object and jq queries execute against it

#### Scenario: YAML file (.yml) is parsed as YAML
- **WHEN** jqpp is launched with a `.yml` file containing a valid YAML sequence
- **THEN** the input pane shows the JSON-equivalent array

#### Scenario: TOML file (.toml) is parsed as TOML
- **WHEN** jqpp is launched with a `.toml` file containing a valid TOML document
- **THEN** the input is the JSON-equivalent object of that TOML document

#### Scenario: XML file (.xml) is parsed as XML
- **WHEN** jqpp is launched with a `.xml` file containing a valid XML document
- **THEN** the input is the jaq-fmts JSON representation of that XML document

#### Scenario: CBOR file (.cbor) is parsed as CBOR
- **WHEN** jqpp is launched with a `.cbor` file containing a valid CBOR value
- **THEN** the input is the JSON-equivalent value decoded from CBOR

#### Scenario: CSV file (.csv) is parsed as an array of row-arrays
- **WHEN** jqpp is launched with a `.csv` file containing tabular data
- **THEN** the input is a JSON array where each element is a JSON array of the row's field values

#### Scenario: TSV file (.tsv) is parsed as an array of row-arrays
- **WHEN** jqpp is launched with a `.tsv` file containing tab-separated data
- **THEN** the input is a JSON array where each element is a JSON array of the row's field values

#### Scenario: File with unrecognised extension uses existing JSON-first path
- **WHEN** jqpp is launched with a file whose extension is not in the recognised set
- **THEN** the file is parsed as JSON (with raw-string fallback), identical to current behaviour

#### Scenario: JSON file (.json) is unaffected
- **WHEN** jqpp is launched with a `.json` file
- **THEN** the file is parsed as JSON, identical to behaviour before this change

### Requirement: A format parse error for a recognised-extension file is a hard error
The system SHALL return an error and exit before opening the TUI when a file with a recognised format extension fails to parse in that format.

#### Scenario: Malformed YAML file causes exit with error
- **WHEN** jqpp is launched with a `.yaml` file that is not valid YAML
- **THEN** the process prints an error identifying the file and exits with status 1

#### Scenario: Malformed TOML file causes exit with error
- **WHEN** jqpp is launched with a `.toml` file that is not valid TOML
- **THEN** the process prints an error identifying the file and exits with status 1

### Requirement: Multi-value formats collapse to a single value or array
The system SHALL unwrap the parsed value directly when a format produces exactly one value, and wrap in a JSON array when it produces more than one.

#### Scenario: Single-document YAML is used directly
- **WHEN** jqpp is launched with a YAML file containing a single document
- **THEN** the input is the parsed value (not wrapped in an array)

#### Scenario: Multi-document YAML is merged into an array
- **WHEN** jqpp is launched with a YAML file containing multiple `---`-separated documents
- **THEN** the input is a JSON array containing each document as an element

### Requirement: YAML stdin is detected and parsed after JSON parse fails
The system SHALL attempt to parse stdin content as YAML when JSON parsing fails, before falling back to a raw JSON string value.

#### Scenario: Valid YAML piped to stdin
- **WHEN** jqpp is launched with a valid YAML document piped to stdin (no file arguments)
- **THEN** the input is the JSON-equivalent value of that YAML document

#### Scenario: Valid JSON piped to stdin is still parsed as JSON
- **WHEN** jqpp is launched with valid JSON piped to stdin
- **THEN** the JSON path is taken and the input is correct

#### Scenario: Non-JSON non-YAML stdin still falls back to string
- **WHEN** jqpp is launched with plain text piped to stdin that is neither JSON nor YAML
- **THEN** the input is a JSON string containing the trimmed text, preserving existing fallback behaviour
