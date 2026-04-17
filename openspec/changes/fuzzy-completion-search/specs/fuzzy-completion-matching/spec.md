## ADDED Requirements

### Requirement: Fuzzy matches are subsequence-based
The system SHALL consider a completion label a fuzzy match when all characters of the typed token appear in the label in order (subsequence), case-insensitively.

#### Scenario: Token is a subsequence of label
- **WHEN** the user has typed `upcase` and a builtin label is `ascii_upcase`
- **THEN** `ascii_upcase` is a fuzzy match for `upcase`

#### Scenario: Token characters appear out of order
- **WHEN** the typed token characters do not all appear in order in the label
- **THEN** that label is NOT returned as a fuzzy match

#### Scenario: Empty token produces no fuzzy results
- **WHEN** the token is empty (cursor after `.`, `|`, or space)
- **THEN** no fuzzy pass is run; only the existing exact-match results are shown

### Requirement: Fuzzy results are scored and ranked
The system SHALL rank fuzzy matches by a score that rewards contiguous character runs and earlier first-match positions, with higher-scoring matches appearing first.

#### Scenario: Contiguous match scores higher
- **WHEN** two labels both match the token as a subsequence but one has the matched characters adjacent and the other has them spread out
- **THEN** the label with adjacent (contiguous) matched characters appears earlier in the fuzzy results

#### Scenario: Earlier match scores higher
- **WHEN** two labels have equally contiguous matches but one starts matching at an earlier position
- **THEN** the label where the match starts earlier appears first

### Requirement: Exact prefix matches always precede fuzzy matches
The system SHALL display exact prefix matches (labels beginning with the token) before any fuzzy-only matches in the dropdown.

#### Scenario: Mixed exact and fuzzy results
- **WHEN** the token `sel` matches `select` as an exact prefix and `ascii_upcase` does not match at all, but `split` also matches as exact prefix
- **THEN** `select` and `split` appear first, followed only by any fuzzy candidates that are not already in the exact list

#### Scenario: Fuzzy duplicate of exact match suppressed
- **WHEN** a label is already present in the exact-match results
- **THEN** it does NOT appear again in the fuzzy section

### Requirement: Fuzzy matches are visually marked
The system SHALL visually distinguish fuzzy-matched suggestions from exact-match suggestions so users can identify which results are approximate.

#### Scenario: Fuzzy item detail prefixed with ~
- **WHEN** a completion item appears as a fuzzy (non-prefix) match
- **THEN** its detail string is prefixed with `~` (e.g. `~string fn`) in the dropdown

#### Scenario: Exact match detail unchanged
- **WHEN** a completion item appears as an exact prefix match
- **THEN** its detail string is shown without modification

### Requirement: Fuzzy matching applies to builtins and json-context fields
The system SHALL apply fuzzy matching to jq builtin completions and JSON field-path completions. LSP completions SHALL be passed through as-is without additional fuzzy processing.

#### Scenario: Fuzzy finds builtin by middle of name
- **WHEN** the user types `str` and no builtin starts with `str`
- **THEN** builtins containing `str` as a subsequence (e.g. `tostring`, `startswith`) appear as fuzzy suggestions

#### Scenario: Fuzzy finds JSON field by fragment
- **WHEN** the user types `nm` and a JSON field is `customer_name`
- **THEN** `customer_name` appears as a fuzzy suggestion

#### Scenario: LSP items not fuzzy-processed
- **WHEN** LSP completions are present
- **THEN** they are merged into results as-is, without additional fuzzy scoring or filtering
