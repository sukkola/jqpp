## MODIFIED Requirements

### Requirement: Strategy assignment per function
The system SHALL assign extraction strategy by function family. For `contains`, `FullString` strategy is active only for string or unknown runtime types; array/object `contains` paths are handled by type-aware parameter suggestions.

#### Scenario: Prefix strategy functions
- **WHEN** `string_param_context` is called for `startswith(` or `ltrimstr(`
- **THEN** returned context has `strategy = Prefix`

#### Scenario: Suffix strategy functions
- **WHEN** `string_param_context` is called for `endswith(` or `rtrimstr(`
- **THEN** returned context has `strategy = Suffix`

#### Scenario: Internal strategy for split
- **WHEN** `string_param_context` is called for `split(`
- **THEN** returned context has `strategy = Internal`

#### Scenario: FullString strategy for index/rindex/indices
- **WHEN** `string_param_context` is called for `index(`, `rindex(`, or `indices(`
- **THEN** returned context has `strategy = FullString`

#### Scenario: contains uses FullString only for string/unknown type
- **WHEN** `string_param_context("contains(", Some("string"))` is evaluated
- **THEN** returns a context with `strategy = FullString`
- **AND WHEN** `string_param_context("contains(", Some("array" | "object"))` is evaluated
- **THEN** returns `None`

### Requirement: string-parameter functions insert empty parens
String-parameter functions SHALL insert with empty parens so the cursor lands in the argument position and follow-up suggestions can drive value selection.

#### Scenario: split insert-text is empty parens
- **WHEN** `get_completions("spl", None)` is called
- **THEN** `split` inserts as `split()`

#### Scenario: startswith insert-text is empty parens
- **WHEN** `get_completions("start", Some("string"))` is called
- **THEN** `startswith` inserts as `startswith()`

#### Scenario: contains insert-text is empty parens
- **WHEN** `get_completions("cont", Some("string" | "array" | "object"))` is called
- **THEN** `contains` inserts as `contains()`
