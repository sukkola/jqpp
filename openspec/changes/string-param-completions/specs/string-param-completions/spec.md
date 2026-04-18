## ADDED Requirements

### Requirement: string_param_context detects cursor inside string-param functions
The system SHALL detect when the query cursor is inside the argument parens of a recognised string-parameter function and return a `StringParamCtx` describing the function, strategy, context path, and inner prefix.

#### Scenario: Cursor inside split parens — bare
- **WHEN** `string_param_context("split(")` is called
- **THEN** returns `Some(StringParamCtx { fn_name: "split", strategy: Internal, context_path: ".", inner_prefix: "" })`

#### Scenario: Cursor inside startswith with partial content
- **WHEN** `string_param_context("startswith(\"shi")` is called (cursor after the opening quote)
- **THEN** returns `Some` with `inner_prefix = "shi"` (leading quote stripped)

#### Scenario: After pipe — context path extracted correctly
- **WHEN** `string_param_context(".orders[].order_status | split(\"_")` is called
- **THEN** returns `Some` with `context_path = ".orders[].order_status"` and `inner_prefix = "_"`

#### Scenario: Cursor after closing paren — no context
- **WHEN** `string_param_context("split(\"-\")")` is called
- **THEN** returns `None` — cursor is past the closing `)`

#### Scenario: Excluded regex function
- **WHEN** `string_param_context("test(\"")` is called
- **THEN** returns `None` — `test` is not a string-literal-param function

#### Scenario: Empty query
- **WHEN** `string_param_context("")` is called
- **THEN** returns `None`

#### Scenario: Non-function context
- **WHEN** `string_param_context(".")` is called
- **THEN** returns `None`

### Requirement: Strategy assignment per function
The system SHALL assign the correct extraction strategy to each recognised function.

#### Scenario: Prefix strategy functions
- **WHEN** `string_param_context` is called for `startswith(` or `ltrimstr(`
- **THEN** the returned context has `strategy = Prefix`

#### Scenario: Suffix strategy functions
- **WHEN** `string_param_context` is called for `endswith(` or `rtrimstr(`
- **THEN** the returned context has `strategy = Suffix`

#### Scenario: Internal strategy for split
- **WHEN** `string_param_context` is called for `split(`
- **THEN** the returned context has `strategy = Internal`

#### Scenario: FullString strategy for contains, index, rindex, indices
- **WHEN** `string_param_context` is called for `contains(`, `index(`, `rindex(`, or `indices(`
- **THEN** the returned context has `strategy = FullString`

### Requirement: Prefix extraction from string values
The system SHALL extract prefix candidates by tokenising on delimiter characters and collecting leading tokens.

#### Scenario: Single delimiter — leading token
- **WHEN** prefix candidates are extracted from `["CUST-42", "CUST-17"]`
- **THEN** results include `"CUST"` and do not include delimiter-attached `"CUST-"`

#### Scenario: Multiple distinct prefixes
- **WHEN** prefix candidates are extracted from `["shipped", "processing", "delivered"]`
- **THEN** results include the full strings as candidates (no delimiter → whole string is the prefix)

#### Scenario: Deduplicated and sorted
- **WHEN** prefix candidates are extracted from a set with repeated prefixes
- **THEN** the result Vec is deduplicated and sorted lexicographically

### Requirement: Suffix extraction from string values
The system SHALL extract suffix candidates by taking trailing delimiter-bounded tokens.

#### Scenario: Email suffixes
- **WHEN** suffix candidates are extracted from `["alice@example.com", "mikko@example.com"]`
- **THEN** results include `"com"`, `"example.com"`, and `"@example.com"` as suffix candidates

#### Scenario: No delimiter — whole string is the suffix
- **WHEN** suffix candidates are extracted from `["shipped", "delivered"]`
- **THEN** results include the full strings

### Requirement: Internal substring extraction from string values
The system SHALL extract internal separator/substring candidates for `split` and substring functions.

#### Scenario: Recurring single-char delimiter appears as candidate
- **WHEN** internal candidates are extracted from `["CUST-42", "ORD-001", "STORE-001"]`
- **THEN** `"-"` is a candidate (appears in all strings as a separator)

#### Scenario: Single-char delimiters that appear in only one string are excluded for split
- **WHEN** internal candidates for `split` are extracted from a set where `"."` appears in only one string
- **THEN** `"."` is NOT suggested (appears in fewer than 2 source strings)

#### Scenario: FullString strategy returns whole strings
- **WHEN** full-string candidates are extracted from `["shipped", "processing", "delivered"]`
- **THEN** results are `["delivered", "processing", "shipped"]` (sorted, deduplicated)

### Requirement: Completions filter by inner_prefix
The system SHALL filter candidates using the inner_prefix the user has typed.

#### Scenario: Prefix filter — startswith
- **WHEN** `get_completions("startswith(\"shi", &json!(["shipped", "processing"]))` is called
- **THEN** only candidates starting with `"shi"` are returned (e.g. `"shipped"` or `"shi"`)

#### Scenario: Empty inner_prefix returns all candidates
- **WHEN** `get_completions("split(", &json!(["a-b", "c-d"]))` is called
- **THEN** all extracted separator candidates are returned unfiltered

#### Scenario: Suffix filter matches from the end
- **WHEN** `get_completions("endswith(\"com", &json!(["alice@corp.com"]))` is called
- **THEN** candidates whose labels end with `"com"` are returned (e.g. `"com"`, `"corp.com"`, `"@corp.com"`)

#### Scenario: Exact before fuzzy
- **WHEN** exact and fuzzy candidates both exist
- **THEN** exact candidates appear first and fuzzy candidates (`detail` prefixed with `~`) are appended after exact matches

#### Scenario: Shortest-first ordering
- **WHEN** exact candidates are returned
- **THEN** candidates are ordered by shortest length first, then lexicographically

### Requirement: Insert-text wraps candidate in double quotes
The system SHALL produce insert-text that replaces the whole query_prefix and wraps the selected candidate in double quotes.

#### Scenario: split candidate insert-text
- **WHEN** the candidate `"-"` is selected for `"split("`
- **THEN** insert-text is `"split(\"-\")"`

#### Scenario: startswith candidate insert-text
- **WHEN** the candidate `"shipped"` is selected for `"startswith(\"shi"`
- **THEN** insert-text is `"startswith(\"shipped\")"`

#### Scenario: Candidates containing double quotes are escaped
- **WHEN** a source string contains `"` characters
- **THEN** the insert-text uses `\"` escaping so the result is valid jq

### Requirement: String values collected from JSON context
The system SHALL collect string values from the resolved JSON at the context path.

#### Scenario: Context is a string scalar
- **WHEN** the resolved value is `"shipped"`
- **THEN** the string set is `["shipped"]`

#### Scenario: Context is an array of mixed types
- **WHEN** the resolved value is `["shipped", 42, null, "processing"]`
- **THEN** only string elements are collected: `["shipped", "processing"]`

#### Scenario: Context is an object with string values
- **WHEN** the resolved value is `{"a": "foo", "b": 1, "c": "bar"}`
- **THEN** string values `["foo", "bar"]` are collected (non-strings skipped)

#### Scenario: Context path not found — no completions
- **WHEN** the context path resolves to nothing in the input
- **THEN** no string-param completions are returned

#### Scenario: Context resolves to non-string scalar
- **WHEN** the resolved value is `42` or `null` or `true`
- **THEN** no string-param completions are returned

### Requirement: String source capped at 500 entries
The system SHALL collect at most 500 source strings to bound worst-case extraction time.

#### Scenario: Large array capped
- **WHEN** the context resolves to an array of 10000 strings
- **THEN** only the first 500 are used for candidate extraction

### Requirement: Excluded functions do not trigger string-param completions
The system SHALL NOT return string-param completions for regex or format-string functions.

#### Scenario: test excluded
- **WHEN** the query prefix is `test("`
- **THEN** `string_param_context` returns `None` and no string-param completions fire

#### Scenario: match excluded
- **WHEN** the query prefix is `match("`
- **THEN** no string-param completions

#### Scenario: strptime excluded
- **WHEN** the query prefix is `strptime("`
- **THEN** no string-param completions

#### Scenario: gsub excluded (multi-arg)
- **WHEN** the query prefix is `gsub("pat"`
- **THEN** no string-param completions

## MODIFIED Requirements

### Requirement: Builtin insert-text for string-param functions uses empty parens
Functions previously inserted with a placeholder quoted argument SHALL now insert with empty parens, placing the cursor inside for immediate string-param completion.

#### Scenario: split insert-text is empty parens
- **WHEN** `get_completions("spl", None)` is called
- **THEN** the `split` item has `insert_text = "split()"`

#### Scenario: startswith insert-text is empty parens
- **WHEN** `get_completions("start", Some("string"))` is called
- **THEN** `startswith` has `insert_text = "startswith()"`

#### Scenario: contains insert-text is empty parens
- **WHEN** `get_completions("cont", None)` is called
- **THEN** `contains` has `insert_text = "contains()"`

### Requirement: Tab and Enter behavior inside string-param contexts
Tab SHALL extend the currently typed argument toward the next meaningful boundary, and Enter SHALL commit the currently typed argument as a valid quoted call.

#### Scenario: Prefix extension on Tab
- **WHEN** typing `startswith("A` with candidates including `"Alice Smith"`
- **THEN** Tab extends to `startswith("Alice` and next Tab can extend to `startswith("Alice Smith`

#### Scenario: Suffix extension on Tab
- **WHEN** typing `endswith("com` with candidates including `"corp.com"` and `"@corp.com"`
- **THEN** first Tab extends to `endswith("corp.com` and next Tab extends to `endswith("@corp.com`

#### Scenario: Enter commits partial value
- **WHEN** typing `startswith("Ali` and pressing Enter
- **THEN** query becomes `startswith("Ali")` and cursor moves to the end of the committed call

### Requirement: Format Operator Restrictions (@tsv, @csv)
The system SHALL only suggest `@tsv` and `@csv` format operators when the input type matches `"array_scalars"`.

#### Scenario: @tsv excluded for array of objects
- **WHEN** `get_completions("", Some("array"))` is called (where `"array"` indicates objects/nested content)
- **THEN** `@tsv` is NOT returned in the suggestions

#### Scenario: @tsv suggested for array of scalars
- **WHEN** `get_completions("", Some("array_scalars"))` is called
- **THEN** `@tsv` is returned in the suggestions

### Requirement: Pipe Prefix Evaluation for Suggestions
The system SHALL evaluate the query prefix preceding a pipe to determine the correct JSON context for suggestions.

#### Scenario: Suggestions in object constructor after complex pipe
- **WHEN** the query is `.users | sort_by([.role, .email])[] | {`
- **THEN** the expression `.users | sort_by([.role, .email])[]` is evaluated
- **AND** fields like `"role"` and `"email"` are suggested within the brace context
