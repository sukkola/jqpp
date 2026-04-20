## ADDED Requirements

### Requirement: `has` function acceptance starts empty and param suggestions are type-aware
The system SHALL insert `has()` on function acceptance and provide type-aware suggestions inside the parameter: object keys for object input and integer indices for array input.

#### Scenario: `has` acceptance inserts empty parens
- **WHEN** `get_completions("has", Some("object"))` or `get_completions("has", Some("array"))` is called
- **THEN** the selected `has` item inserts as `has()`

#### Scenario: `has` param with object input
- **WHEN** the query prefix is `has(` and input is `{"name":"Alice","age":30}`
- **THEN** completions include `has("name")` and `has("age")`

#### Scenario: `has` param with array input
- **WHEN** the query prefix is `has(` and input is `["a","b","c"]`
- **THEN** completions include `has(0)`, `has(1)`, `has(2)`

#### Scenario: `has` param after pipe context
- **WHEN** the query prefix is `.users[] | has(` and `.users` is an array of objects
- **THEN** key completions are offered from a `.users[]` element

### Requirement: `contains` function acceptance starts empty and param suggestions are type-aware
The system SHALL insert `contains()` on function acceptance and provide parameter suggestions based on runtime input type.

#### Scenario: `contains` acceptance inserts empty parens
- **WHEN** `get_completions("contains", Some("string" | "array" | "object"))` is called
- **THEN** the selected `contains` item inserts as `contains()`

#### Scenario: string input suggests string candidates
- **WHEN** the query prefix is `contains("` and input is a string or string-bearing context
- **THEN** suggestions include relevant string values (including tokenized candidates)

#### Scenario: array-of-scalars input suggests array values
- **WHEN** the query prefix is `contains([` and input is `["hello world","foo","bar baz","123"]`
- **THEN** suggestions include scalar values from the array for progressive selection

#### Scenario: array-of-objects input suggests object keys then values
- **WHEN** the query prefix is `.orders[]|contains({` and input contains multiple order objects
- **THEN** key suggestions are shown first
- **AND WHEN** a key is selected and cursor is at its value position
- **THEN** value suggestions include values for that key across all matching objects (not only the first object)

#### Scenario: no duplicate `contains` entries
- **WHEN** `get_completions("cont", Some("object"))` is called
- **THEN** exactly one `contains` item appears in completion results

### Requirement: jq/jaq support gaps are surfaced in builtin details
The system SHALL annotate known support differences in builtin details.

#### Scenario: `debug` is bare insert text
- **WHEN** `get_completions("debug", None)` is called
- **THEN** `debug` inserts as `debug` (no argument placeholder)

#### Scenario: `@csv` and `@tsv` mention extension status
- **WHEN** `get_completions("@csv", None)` or `get_completions("@tsv", None)` is called
- **THEN** detail mentions jqpp extension / limited jaq support

#### Scenario: `input` and `inputs` mention limitations
- **WHEN** `get_completions("input", None)` or `get_completions("inputs", None)` is called
- **THEN** detail marks limited jaq support
