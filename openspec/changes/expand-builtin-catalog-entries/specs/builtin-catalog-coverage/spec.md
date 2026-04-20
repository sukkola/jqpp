## ADDED Requirements

### Requirement: Dedup key allows same-name multi-argument entries
`get_completions` SHALL deduplicate by `(name, insert_text)` pair rather than `name` alone, so that a function with multiple argument forms (e.g. `flatten` and `flatten(1)`) can both appear in the output.

#### Scenario: Two same-name entries with different insert texts both appear
- **WHEN** `get_completions("flatten", None)` is called
- **THEN** the result contains two items with label `"flatten"`: one with insert text `"flatten"` and one with insert text `"flatten(1)"`

#### Scenario: Truly identical entries are still deduplicated
- **WHEN** the BUILTINS slice contains two rows with the same name AND the same insert_text
- **THEN** `get_completions` emits only one item for that pair

#### Scenario: Existing no-duplicate guarantee holds for unambiguous names
- **WHEN** `get_completions("", None)` is called
- **THEN** no `(label, insert_text)` pair appears more than once in the output

---

### Requirement: flatten exposes depth-limited form
The catalog SHALL contain a second entry for `flatten` with insert text `flatten(1)` and `InputType::Array`.

#### Scenario: flatten(1) appears for array input
- **WHEN** `get_completions("flatten", Some("array"))` is called
- **THEN** the result contains an item with insert text `"flatten(1)"` and detail `"flatten N levels deep"`

#### Scenario: flatten bare form is still present
- **WHEN** `get_completions("flatten", Some("array"))` is called
- **THEN** the result also contains an item with insert text `"flatten"` (no arguments)

#### Scenario: flatten entries do not appear for non-array input
- **WHEN** `get_completions("flatten", Some("string"))` is called
- **THEN** the result is empty (no flatten entries)

---

### Requirement: range exposes two- and three-argument forms
The catalog SHALL contain entries for `range(0; 10)` and `range(0; 10; 2)` alongside the existing `range(10)`, all with `InputType::Any`.

#### Scenario: All three range forms appear with no type filter
- **WHEN** `get_completions("range", None)` is called
- **THEN** the result contains items with insert texts `"range(10)"`, `"range(0; 10)"`, and `"range(0; 10; 2)"`

#### Scenario: range forms appear for any input type
- **WHEN** `get_completions("range", Some("object"))` is called
- **THEN** the result contains all three range forms (they are `InputType::Any`)

#### Scenario: range detail strings distinguish forms
- **WHEN** `get_completions("range", None)` is called
- **THEN** the item with insert text `"range(0; 10; 2)"` has a detail string that mentions step or stride

---

### Requirement: paths exposes predicate form
The catalog SHALL contain a second entry for `paths` with insert text `paths(scalars)` and `InputType::Any`.

#### Scenario: paths(scalars) appears alongside bare paths
- **WHEN** `get_completions("paths", None)` is called
- **THEN** the result contains an item with insert text `"paths"` and an item with insert text `"paths(scalars)"`

#### Scenario: paths(scalars) detail string describes predicate filtering
- **WHEN** `get_completions("paths", None)` is called
- **THEN** the item with insert text `"paths(scalars)"` has a detail string that references filtering or predicates

---

### Requirement: recurse exposes safe argument form
The catalog SHALL contain a second entry for `recurse` with insert text `recurse(.[]?)` and `InputType::Any`.

#### Scenario: recurse(.[]?) appears alongside bare recurse
- **WHEN** `get_completions("recurse", None)` is called
- **THEN** the result contains an item with insert text `"recurse"` and an item with insert text `"recurse(.[]?)"`

#### Scenario: recurse(.[]?) detail string mentions mixed-type safety
- **WHEN** `get_completions("recurse", None)` is called
- **THEN** the item with insert text `"recurse(.[]?)"` has a detail string that references safe traversal or error suppression

---

### Requirement: strptime catalog covers common format strings
The catalog SHALL contain additional entries for `strptime` with insert texts for common date formats, all with `InputType::String`.

#### Scenario: strptime ISO datetime format appears
- **WHEN** `get_completions("strptime", Some("string"))` is called
- **THEN** the result contains an item with insert text `"strptime(\"%Y-%m-%dT%H:%M:%S\")"`

#### Scenario: strptime day/month/year format appears
- **WHEN** `get_completions("strptime", Some("string"))` is called
- **THEN** the result contains an item with insert text `"strptime(\"%d/%m/%Y\")"`

#### Scenario: strptime time-only format appears
- **WHEN** `get_completions("strptime", Some("string"))` is called
- **THEN** the result contains an item with insert text `"strptime(\"%H:%M:%S\")"`

#### Scenario: strptime entries do not appear for number input
- **WHEN** `get_completions("strptime", Some("number"))` is called
- **THEN** the result is empty

---

### Requirement: strftime catalog covers common format strings
The catalog SHALL contain additional entries for `strftime` with insert texts for common date formats, all with `InputType::Number`.

#### Scenario: strftime ISO datetime format appears
- **WHEN** `get_completions("strftime", Some("number"))` is called
- **THEN** the result contains an item with insert text `"strftime(\"%Y-%m-%dT%H:%M:%SZ\")"`

#### Scenario: strftime time-only format appears
- **WHEN** `get_completions("strftime", Some("number"))` is called
- **THEN** the result contains an item with insert text `"strftime(\"%H:%M:%S\")"`

#### Scenario: strftime year/month/day with time format appears
- **WHEN** `get_completions("strftime", Some("number"))` is called
- **THEN** the result contains an item with insert text `"strftime(\"%Y/%m/%d %H:%M\")"`

#### Scenario: strftime entries do not appear for string input
- **WHEN** `get_completions("strftime", Some("string"))` is called
- **THEN** the result is empty (strftime is number-only)

---

### Requirement: values detail string matches jaq semantics
The `values` catalog entry detail string SHALL describe jaq's actual behavior (`select(. != null)` — filters nulls from a stream) rather than the misleading "values as array".

#### Scenario: values detail string reflects jaq filter semantics
- **WHEN** `get_completions("values", None)` is called
- **THEN** the item with label `"values"` has a detail string that does NOT contain "values as array"
- **THEN** the detail string references null filtering or `select`

---

### Requirement: maturity docs for resolved items move to completed
All maturity files whose outstanding work is fully addressed by this change SHALL be moved to `docs/maturity/completed/` and removed from the active tables in `docs/maturity/README.md`.

#### Scenario: flatten maturity file is in completed after implementation
- **WHEN** the change is applied
- **THEN** `docs/maturity/completed/flatten.md` exists
- **THEN** `docs/maturity/flatten.md` does not exist

#### Scenario: range maturity file is in completed after implementation
- **WHEN** the change is applied
- **THEN** `docs/maturity/completed/range.md` exists

#### Scenario: paths maturity file is in completed after implementation
- **WHEN** the change is applied
- **THEN** `docs/maturity/completed/paths.md` exists

#### Scenario: recurse maturity file is in completed after implementation
- **WHEN** the change is applied
- **THEN** `docs/maturity/completed/recurse.md` exists

#### Scenario: keys-values maturity file is in completed after implementation
- **WHEN** the change is applied
- **THEN** `docs/maturity/completed/keys-values.md` exists

#### Scenario: README active tables no longer list moved functions
- **WHEN** the change is applied
- **THEN** `docs/maturity/README.md` Low complexity list does not reference flatten, range, paths, recurse, or keys-values as open items
