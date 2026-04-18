# param-field-completions Specification

## Purpose
TBD - created by archiving change param-field-completions. Update Purpose after archive.
## Requirements
### Requirement: Field completions inside sort_by parameter
The system SHALL provide field-name completions when the cursor is inside the argument parens of `sort_by`, sourcing field names from the first element of the array that feeds into the function.

#### Scenario: Empty inner prefix offers all element fields
- **WHEN** the query prefix is `sort_by(.` and the JSON input is an array of objects
- **THEN** all field names from the first array element are suggested, each with insert-text `sort_by(.fieldname)`

#### Scenario: Partial inner prefix filters element fields
- **WHEN** the query prefix is `sort_by(.na` and the JSON input is an array of objects with fields `name`, `age`, `namespace`
- **THEN** only fields starting with `na` are suggested (`name`, `namespace`), with insert-texts `sort_by(.name)` style (prefix preserved)

#### Scenario: After a pipe — element fields resolved from pipe context
- **WHEN** the query prefix is `.orders[] | sort_by(.` and the input has `.orders` as an array of objects
- **THEN** field names from `.orders` elements are suggested, with insert-texts `.orders[] | sort_by(.fieldname)`

#### Scenario: Array is empty — no completions
- **WHEN** the query prefix is `sort_by(.` and the JSON input is an empty array `[]`
- **THEN** no field completions are returned

#### Scenario: Array elements are scalars — no completions
- **WHEN** the query prefix is `sort_by(.` and the JSON input is `[1, 2, 3]`
- **THEN** no field completions are returned (scalars have no fields)

### Requirement: Field completions inside group_by parameter
The system SHALL provide field-name completions inside `group_by(…)` using the same element-field resolution as `sort_by`.

#### Scenario: Basic group_by completion
- **WHEN** the query prefix is `group_by(.` and the JSON input is `[{"status": "a", "id": 1}]`
- **THEN** `status` and `id` are suggested with insert-texts `group_by(.status)` and `group_by(.id)`

#### Scenario: Partial prefix inside group_by
- **WHEN** the query prefix is `group_by(.st` and the JSON input is `[{"status": "a", "id": 1}]`
- **THEN** only `status` is suggested

### Requirement: Field completions inside unique_by parameter
The system SHALL provide field-name completions inside `unique_by(…)` using element-field resolution.

#### Scenario: Basic unique_by completion
- **WHEN** the query prefix is `unique_by(.` and the JSON input is `[{"name": "a"}, {"name": "b"}]`
- **THEN** `name` is suggested with insert-text `unique_by(.name)`

### Requirement: Field completions inside min_by and max_by parameters
The system SHALL provide field-name completions inside `min_by(…)` and `max_by(…)` using element-field resolution.

#### Scenario: min_by completion
- **WHEN** the query prefix is `min_by(.` and the JSON input is `[{"price": 1.0, "qty": 5}]`
- **THEN** `price` and `qty` are suggested

#### Scenario: max_by completion
- **WHEN** the query prefix is `max_by(.` and the JSON input is `[{"score": 99}]`
- **THEN** `score` is suggested

### Requirement: Field completions inside del parameter
The system SHALL provide field-name completions inside `del(…)`, sourcing field names from the current input value (not array elements).

#### Scenario: del with object input
- **WHEN** the query prefix is `del(.` and the JSON input is `{"name": "alice", "age": 30}`
- **THEN** `name` and `age` are suggested with insert-texts `del(.name)` and `del(.age)`

#### Scenario: del with partial prefix
- **WHEN** the query prefix is `del(.ag` and the JSON input is `{"name": "alice", "age": 30}`
- **THEN** only `age` is suggested

#### Scenario: del after a pipe
- **WHEN** the query prefix is `.user | del(.` and the JSON input is `{"user": {"id": 1, "token": "x"}}`
- **THEN** `id` and `token` are suggested with insert-texts `.user | del(.id)` and `.user | del(.token)`

### Requirement: Field completions inside path parameter
The system SHALL provide field-name completions inside `path(…)`, sourcing field names from the current input value.

#### Scenario: path with object input
- **WHEN** the query prefix is `path(.` and the JSON input is `{"a": 1, "b": 2}`
- **THEN** `a` and `b` are suggested with insert-texts `path(.a)` and `path(.b)`

### Requirement: No completions inside non-field-path functions
The system SHALL NOT provide these param-field completions inside function calls that take general filter expressions, including `map`, `select`, `with_entries`, `any`, `all`, `reduce`, `foreach`, `while`, `until`, `walk`.

#### Scenario: map does not trigger param-field completions
- **WHEN** the query prefix is `map(.` and the JSON input is an array of objects
- **THEN** no param-field completions are returned (normal dot-path completions may still fire)

#### Scenario: select does not trigger param-field completions
- **WHEN** the query prefix is `select(.` and the JSON input is an object
- **THEN** no param-field completions specific to the `select` parameter are returned

### Requirement: Nested field paths inside parameters
The system SHALL support nested field paths inside parameters (e.g. `.customer.name`).

#### Scenario: Nested path completion inside sort_by
- **WHEN** the query prefix is `sort_by(.customer.` and the JSON input is `[{"customer": {"name": "a", "id": 1}}]`
- **THEN** `name` and `id` are suggested with insert-texts `sort_by(.customer.name)` and `sort_by(.customer.id)`

#### Scenario: Partial nested path completion
- **WHEN** the query prefix is `sort_by(.customer.na` and the JSON input is `[{"customer": {"name": "alice", "namespace": "x"}}]`
- **THEN** `name` and `namespace` are suggested

### Requirement: Non-field-path input yields no param completions
The system SHALL return no param-field completions when the JSON at the resolved context path is not an object or array-of-objects.

#### Scenario: Context resolves to a string
- **WHEN** the query prefix is `sort_by(.` and the JSON input is `"hello"`
- **THEN** no field completions are returned

#### Scenario: Context path not found
- **WHEN** the query prefix is `.missing | sort_by(.` and the JSON has no `missing` key
- **THEN** no field completions are returned

