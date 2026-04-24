## MODIFIED Requirements

### Requirement: Function acceptance starts from empty argument position
When a function completion is accepted (Tab/Enter), the inserted text SHALL place the cursor at the next user-input position and SHALL NOT prefill semantic placeholder arguments (for example object keys, indices, or sample literals). Function calls with arguments therefore insert as empty parentheses (`fn()`) and rely on follow-up contextual suggestions to guide argument entry. This applies to `select` in addition to `has` and `contains`.

#### Scenario: has inserts empty parens
- **WHEN** the user accepts the `has` completion
- **THEN** the query inserts `has()` and the cursor lands between `(` and `)`

#### Scenario: contains inserts empty parens
- **WHEN** the user accepts the `contains` completion
- **THEN** the query inserts `contains()` and the cursor lands between `(` and `)`

#### Scenario: select inserts empty parens
- **WHEN** the user accepts the `select` completion
- **THEN** the query inserts `select()` and the cursor lands between `(` and `)`

#### Scenario: contextual suggestions appear immediately after function acceptance
- **WHEN** the user is positioned inside `has(` or `contains(` or `select(` after accepting a completion
- **THEN** the dropdown offers context-aware argument suggestions without requiring manual deletion of prefilled placeholders

#### Scenario: select contextual suggestions are condition starters
- **WHEN** the user is positioned inside `select(` after accepting the completion
- **THEN** the dropdown offers condition starter suggestions appropriate for the inferred input type (not generic field paths or builtins)
