## ADDED Requirements

### Requirement: Static builtin catalog
`src/completions/jq_builtins.rs` SHALL maintain a compile-time catalog of ~90 jq builtin functions with name, insert_text, detail, and input type annotation.

#### Scenario: Catalog loaded without I/O
- **WHEN** the application starts
- **THEN** builtin completions are available immediately with no file or network access

### Requirement: Input-type filtering
Each catalog entry is annotated with an `InputType` variant. When a pipe-prefix type is known, only compatible entries are returned.

#### InputType variants
| Variant | Compatible with |
|---|---|
| `Any` | all types |
| `NonBoolean` | string, number, array, object, null (not boolean) |
| `String` | string only |
| `Number` | number only |
| `Array` | array only |
| `Object` | object only |
| `StringOrArray` | string or array |
| `ArrayOrObject` | array or object |

#### Scenario: String context
- **WHEN** pipe-prefix type is `"string"`
- **THEN** `ascii_upcase`, `split`, `ltrimstr`, etc. are included; `sort`, `floor`, `to_entries` are excluded

#### Scenario: Number context
- **WHEN** pipe-prefix type is `"number"`
- **THEN** `floor`, `ceil`, `sqrt`, `fabs`, etc. are included; `ascii_upcase`, `sort` are excluded

#### Scenario: Array context
- **WHEN** pipe-prefix type is `"array"`
- **THEN** `sort`, `unique`, `flatten`, `map`, `first`, `last`, etc. are included; `ascii_upcase` is excluded

#### Scenario: Boolean context
- **WHEN** pipe-prefix type is `"boolean"`
- **THEN** `length` is excluded (boolean has no length in jq); universal functions like `not`, `type`, `empty` are included

#### Scenario: Unknown type (no pipe)
- **WHEN** `cached_pipe_type` is `None`
- **THEN** all catalog entries are returned (no type filtering applied)

### Requirement: length excluded for boolean input
`length` is annotated `InputType::NonBoolean`. Calling `true | length` is a jq runtime error.

#### Scenario: length excluded for boolean
- **WHEN** pipe-prefix evaluates to a boolean
- **THEN** `length` does not appear in the suggestion list

#### Scenario: length appears for all other types
- **WHEN** pipe-prefix evaluates to string, number, array, object, or null
- **THEN** `length` appears in the suggestion list

### Requirement: @base64 only for strings
`@base64` is annotated `InputType::String`. Applying it to non-strings is a jq error.

#### Scenario: @base64 excluded for non-string
- **WHEN** pipe-prefix type is array, object, number, boolean, or null
- **THEN** `@base64` does not appear in the suggestion list

### Requirement: Two-pass ordering
Catalog lookup uses two passes: first collect entries compatible with the specific type, then collect `Any`/`NonBoolean` entries. This ensures type-specific suggestions bubble to the top.

#### Scenario: Type-specific before universal
- **WHEN** pipe-prefix type is `"string"` and token is empty
- **THEN** string-specific functions appear before universal functions in the list

### Requirement: No duplicates in output
The two-pass merge deduplicates by name so that no label appears twice regardless of how many passes match.

#### Scenario: No duplicate labels
- **WHEN** `get_completions` is called for any token and type
- **THEN** all returned labels are unique

### Requirement: insert_text cursor placement for parameterized functions
Parameterized builtins include the argument placeholder in `insert_text` (e.g. `split(",")`, `ltrimstr("")`). On accept, the cursor SHALL land after `("` so the user can type the argument directly.

#### Scenario: Cursor inside string argument
- **WHEN** `split(",")` is accepted
- **THEN** cursor is at column 7 (after `split("`) not column 10 (after `)`)

### Requirement: Control-flow and accumulator builtins included
The catalog SHALL include entries for `reduce`, `foreach`, `until`, `while`, `limit`, `first(expr)`, `last(expr)`, and `range` so that these constructs appear in the dropdown with useful argument scaffolding.

#### Scenario: reduce appears with template
- **WHEN** the user begins typing `red` in a pipe context
- **THEN** `reduce` appears with insert_text `reduce .[] as $x (0; . + $x)`

#### Scenario: String-only builtins excluded from non-string contexts
- **WHEN** pipe-prefix type is `"number"`, `"array"`, or `"boolean"`
- **THEN** `ascii_upcase`, `ascii_downcase`, `ltrimstr`, `rtrimstr`, `split` do NOT appear

#### Scenario: Format operators included as catalog entries
- **WHEN** the user types `@` in a pipe context
- **THEN** `@csv`, `@tsv`, `@base64`, `@base64d`, `@uri`, `@html`, `@sh`, `@json`, `@text` appear in the dropdown
