## ADDED Requirements

### Requirement: Variable names are extracted from as-bindings in the query prefix
The system SHALL scan the current query prefix for `as $name` patterns and collect all distinct bound variable names.

#### Scenario: Single binding found
- **WHEN** the query prefix is `5 as $x |`
- **THEN** the extracted variable names include `x`

#### Scenario: Multiple bindings found
- **WHEN** the query prefix is `.[] as $item | $item.tags[] as $tag |`
- **THEN** the extracted variable names include both `item` and `tag`

#### Scenario: reduce binding found
- **WHEN** the query prefix is `reduce .[] as $acc (0;`
- **THEN** the extracted variable names include `acc`

#### Scenario: foreach binding found
- **WHEN** the query prefix is `foreach .[] as $x (0;`
- **THEN** the extracted variable names include `x`

#### Scenario: No bindings in query
- **WHEN** the query prefix is `.foo | .bar`
- **THEN** no variable names are extracted

#### Scenario: Empty query
- **WHEN** the query prefix is `""`
- **THEN** no variable names are extracted

### Requirement: Typing $ triggers variable name completions
The system SHALL offer completions for all variable names extracted from the current query prefix when the current token starts with `$`.

#### Scenario: $ alone offers all bound variables
- **WHEN** the current query prefix is `5 as $x | 10 as $y | $` (cursor at end)
- **THEN** completions include `$x` and `$y` with `detail = "bound variable"`

#### Scenario: Partial $ prefix filters variables
- **WHEN** the current query prefix is `5 as $foo | 10 as $bar | $f`
- **THEN** only `$foo` is suggested (matches `$f` prefix); `$bar` is not

#### Scenario: No match — no completions
- **WHEN** the current query prefix is `5 as $foo | $z`
- **THEN** no variable completions are returned (no bound name starts with `z`)

#### Scenario: Variable completions appear before builtins
- **WHEN** completions are generated and bound variable names match the current token
- **THEN** variable completions are listed before builtin function completions in the result

### Requirement: Variable completions use $ prefix in insert text
The system SHALL include the `$` sigil in the `insert_text` of variable completions so selecting a variable completion produces valid jq syntax.

#### Scenario: insert_text includes $ sigil
- **WHEN** a variable completion for name `x` is generated
- **THEN** its `insert_text = "$x"` and `label = "$x"`

#### Scenario: detail identifies binding source
- **WHEN** a variable completion is generated
- **THEN** its `detail = "bound variable"`
