## ADDED Requirements

### Requirement: Detect cursor inside select argument
The system SHALL recognise when the cursor is positioned inside the argument of a `select(` call and activate a dedicated select-condition completion context.

#### Scenario: Cursor directly after select open paren
- **WHEN** the query prefix ends with `select(`
- **THEN** select-condition context is active

#### Scenario: Cursor after partial condition inside select
- **WHEN** the query prefix ends with `select(. >` or any partial condition expression
- **THEN** select-condition context is active

#### Scenario: Cursor outside select parens
- **WHEN** the query prefix ends with `select()` (closed paren, cursor past it)
- **THEN** select-condition context is NOT active

### Requirement: Generate condition starters from inferred input type
The system SHALL generate condition starter suggestions based on the type of the value produced by the pipeline preceding `select(`. Starters SHALL be partial expressions the user completes — not pre-filled with example values.

#### Scenario: Number input generates numeric comparisons
- **WHEN** the input flowing into `select(` is of type number (scalar or via `.[]`)
- **THEN** suggestions include starters such as `. > `, `. < `, `. == `, `. >= `, `. <= `, `. != `

#### Scenario: String input generates string condition starters
- **WHEN** the input flowing into `select(` is of type string
- **THEN** suggestions include starters such as `length > `, `startswith(`, `endswith(`, `test(`, `. == `, `. != null`

#### Scenario: Object input generates field-qualified starters
- **WHEN** the input flowing into `select(` is an object with known keys
- **THEN** suggestions include field-specific starters like `.age > `, `.name == `, `.name != null` derived from the object's actual keys and their value types

#### Scenario: Array input generates length-based starters
- **WHEN** the input flowing into `select(` is an array
- **THEN** suggestions include starters such as `length > `, `length == `, `. != null`

#### Scenario: Unknown or null input falls back to generic starters
- **WHEN** the input type cannot be determined or is null
- **THEN** suggestions include generic starters: `. != null`, `. == null`, `type == `

### Requirement: Filter condition starters by typed prefix
The system SHALL filter the generated condition starters by what the user has already typed inside the `select(` parens, matching by prefix or fuzzy match.

#### Scenario: Empty inner prefix shows all starters
- **WHEN** the cursor is at `select(` with nothing typed inside
- **THEN** all condition starters for the inferred input type are shown

#### Scenario: Partial typed prefix narrows starters
- **WHEN** the user has typed `le` inside `select(`
- **THEN** only starters beginning with or fuzzy-matching `le` (e.g. `length > `) appear

### Requirement: Condition starters are partial — user fills in operand
Each condition starter suggestion SHALL end at the operator boundary so the user types the value. Starters SHALL NOT contain example numeric or string literals.

#### Scenario: Number starter leaves value blank
- **WHEN** the user accepts `. > ` for a number context
- **THEN** the inserted text is `. > ` and the cursor is positioned after the space, ready for the user to type a value

#### Scenario: Field starter leaves value blank
- **WHEN** the user accepts `.age > ` for an object with a numeric `age` field
- **THEN** the inserted text is `.age > ` and the cursor is positioned after the space

### Requirement: Evaluate pipe context to determine input type
The system SHALL evaluate the pipeline segment before `select(` against the current JSON input to determine what value flows into `select`. If evaluation fails, the system SHALL fall back to inferring from the raw input.

#### Scenario: Array iteration resolves to element type
- **WHEN** the query is `.[] | select(` and the input is `[1, 2, 3]`
- **THEN** the inferred type is number and numeric starters are offered

#### Scenario: Object field access resolves to field value type
- **WHEN** the query is `.users[] | select(` and input has a `users` array of objects
- **THEN** the inferred type is object and field-qualified starters are offered

#### Scenario: Evaluation failure falls back gracefully
- **WHEN** the pipeline prefix cannot be evaluated (incomplete expression)
- **THEN** generic condition starters are offered rather than no suggestions
