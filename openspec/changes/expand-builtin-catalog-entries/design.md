## Context

The builtin catalog in `src/completions/jq_builtins.rs` is a `const` slice of `(name, insert_text, detail, InputType)` tuples. At completion time, `get_completions(token, input_type)` runs two passes over this slice: type-specific entries first (anything other than `InputType::Any`), then universal entries — ensuring the most relevant suggestions surface at the top of the list. Deduplication by name is done inside `get_completions` so the same function name can appear on multiple rows with different `InputType` values and only the first matching row is returned to the UI.

This change makes no modifications to `get_completions`, `InputType`, `json_context.rs`, or any other module. Every addition is a new tuple in the `BUILTINS` constant, plus one detail string fix.

The one change that *could* be architectural — a `StaticCandidates` strategy for format-string arguments (`strptime`, `strftime`) — is **out of scope**. The maturity docs note that this would require adding a new strategy to `json_context.rs`. This change instead adds multiple catalog entries with different hard-coded format strings so users can discover them through autocomplete, deferring the in-argument-position strategy to a later proposal.

## Goals / Non-Goals

**Goals:**
- Surface `flatten(1)`, `range(0; 10)`, `range(0; 10; 2)`, `paths(scalars)`, and `recurse(.[]?)` as catalog entries discoverable via autocomplete.
- Add several common date-format catalog entries for `strptime` and `strftime` (e.g. `"%Y-%m-%dT%H:%M:%S"`, `"%d/%m/%Y"`, `"%H:%M:%S"`, `"%Y/%m/%d %H:%M"`) so users can discover format variants through the suggestion list. Note: the proper in-argument solution (StaticCandidates strategy) is Medium complexity and out of scope; these are catalog rows only as a stopgap.
- Fix the `values` detail string to correctly describe jaq semantics: `select(. != null)` (filter nulls from stream), not "values as array".
- Optionally add a second object-update example for `until` and `while` (low value, append-only).
- Move the now-resolved maturity files to `docs/maturity/completed/` and update `docs/maturity/README.md`.

**Non-Goals:**
- A `StaticCandidates` in-argument strategy for `strptime`/`strftime` (would require `json_context.rs` changes — deferred).
- Changes to `get_completions` algorithm or `InputType` variants.
- Any new data-driven (live JSON) completions.
- Changes to `suggestions.rs`, `lsp.rs`, or `executor.rs`.

## Decisions

### 1. Multiple catalog rows instead of dedup bypass

The existing dedup logic drops the second row for the same name. The catalog already exploits this intentionally for `has` (two rows: `Object` and `Array`) and `contains` (three rows: `String`, `Array`, `Object`). The dedup key is `name` — the first matching row wins.

For functions like `flatten` or `range`, both forms (`flatten` and `flatten(1)`) have the **same name**. If placed consecutively in `BUILTINS`, the dedup logic would only return the first one.

**Decision:** The entry for the argument form uses a **distinct name prefix approach** — there is no distinct name. Instead, both entries are present under the same name but the dedup logic means only one will ever appear.

Wait — re-reading `get_completions`:

```rust
if seen.insert(name) {
    out.push(...)
}
```

This deduplicates by `name` string. `"flatten"` and `"flatten"` are the same name, so only the first row is ever returned.

**Revised decision:** The dedup-by-name behavior must be changed for this feature to work. The options are:

- **Option A**: Change dedup key from `name` to `(name, insert_text)`. This means all distinct `insert_text` rows for the same name appear in the output.
- **Option B**: Change dedup key to `insert_text` only, so truly identical insert texts are deduplicated but argument-form variants are distinct.
- **Option C**: Add a separate `label` field so `flatten` and `flatten(1)` are distinct labels. Requires changing the `CompletionItem` struct or BUILTINS tuple shape.

**Choice: Option A** — dedup by `(name, insert_text)`. This is minimal: it still deduplicates truly identical entries, and allows the same function name to appear multiple times with different insert texts. The `label` field in the TUI shows the function name regardless, so the user sees `flatten` listed twice in the completion menu — once as bare `flatten` and once as `flatten(1)`. This matches how other tools (e.g. VSCode IntelliSense) surface function overloads: same name, different signatures.

This requires a one-line change to the `seen.insert(name)` call: `seen.insert((name, insert_text))`.

### 2. strptime/strftime: multiple catalog rows, not in-argument strategy

Rather than implementing a new `StaticCandidates` argument-position strategy, four additional catalog rows are added for each function — one per common format string. The `insert_text` for each row is `strptime("%Y-%m-%dT%H:%M:%S")` etc., with the format string already populated. Users see the full call in the autocomplete list and can select whichever format they want.

This has a minor UX downside: users see `strptime` listed five times in the suggestion menu. However, the label shows the detail string (which names the format), and the approach requires zero infrastructure change.

### 3. values detail fix

The current detail string is `"values as array"`, which matches standard jq (`values` iterates `.[]`). In jaq, `values` is defined as `select(. != null)` — it filters a stream, not an array iteration. The fix is to update the detail string to `"filter nulls (select(!= null))"`. No semantic or behavior change; purely documentation.

### 4. Maturity file lifecycle

The Low-complexity items resolved by this change have maturity docs in `docs/maturity/`. Once the catalog entries are added, those docs no longer track outstanding work — they should move to `docs/maturity/completed/`. The README index must be updated to remove the moved files from the active tables and Low-complexity summary list.

Files to move:
- `flatten.md`
- `range.md`
- `paths.md`
- `recurse.md`
- `strptime.md`
- `strftime.md`
- `keys-values.md`
- `until-while.md` *(if the optional entries are added)*

## Risks / Trade-offs

**[Dedup key change] `seen.insert((name, insert_text))` means tests expecting exactly one entry per name for `flatten`, `range`, etc. may need updating.**
→ Mitigation: the existing `no_duplicates_in_output` test checks `label` deduplication and will fail because `flatten` now appears twice. Update the test to allow multiple entries per name while still ensuring no truly identical `(name, insert_text)` pair is emitted twice.

**[strptime/strftime spam] Five rows per function increases menu noise.**
→ Mitigation: entries appear together in the menu. The detail string differentiates them. Acceptable for a catalog-only change; a better UX with in-argument completions can replace these rows in a future change.

**[values detail] Changing "values as array" to "filter nulls (select(!= null))" may surprise users expecting standard jq semantics.**
→ Acceptable: jqpp explicitly targets jaq. The detail string should match the runtime engine's behavior.

## Migration Plan

1. Change `seen.insert(name)` → `seen.insert((name, insert_text))` in `get_completions`.
2. Add new catalog entries to `BUILTINS` const.
3. Fix `values` detail string.
4. Update existing test `no_duplicates_in_output` to check `(label, insert_text)` uniqueness instead of `label` uniqueness.
5. Add new tests for each added entry (see spec for WHEN/THEN scenarios).
6. Run `cargo test` — all tests must pass.
7. Move maturity files to `docs/maturity/completed/` and update README.

No rollback concern — these are additive catalog changes with no runtime behavior impact.

## Open Questions

*(none — all decisions made above)*
