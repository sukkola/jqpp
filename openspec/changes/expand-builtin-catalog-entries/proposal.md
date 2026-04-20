## Why

The builtin catalog in `jq_builtins.rs` exposes only the zero-argument or simplest-argument form of several functions, leaving useful variants entirely invisible to completion. Users who don't know `flatten(1)`, `range(0; 10)`, `paths(scalars)`, or `recurse(.[]?)` exist have no way to discover them from the TUI. Additionally, the `values` detail string misdescribes jaq's actual semantics, and `strptime`/`strftime` offer only one example format string when several common patterns are worth surfacing.

## What Changes

- **`flatten`**: Add `flatten(1)` as a second catalog entry alongside bare `flatten`.
- **`range`**: Add two-argument `range(0; 10)` and three-argument `range(0; 10; 2)` entries alongside `range(10)`.
- **`paths`**: Add `paths(scalars)` entry alongside bare `paths` to expose the predicate form.
- **`recurse`**: Add `recurse(.[]?)` entry alongside bare `recurse` — the safe form for mixed-type trees.
- **`strptime` / `strftime`**: Add catalog entries for common date format strings (`"%Y-%m-%dT%H:%M:%S"`, `"%d/%m/%Y"`, `"%H:%M:%S"`, `"%Y/%m/%d %H:%M"`) as additional entries alongside the existing `"%Y-%m-%d"` insert text.
- **`values`**: Fix the detail string — jaq defines `values` as `select(. != null)`, not "values as array"; the current description misleads users.
- **`until` / `while`** *(optional)*: Add object-iteration example entries to complement the existing numeric examples.
- **`docs/maturity`**: Move the now-resolved maturity files for these functions to `completed/` and update `README.md`.

## Capabilities

### New Capabilities

- `builtin-catalog-coverage`: The builtin catalog SHALL expose all commonly-used argument forms of every supported function, not just the simplest form. Each distinct argument form is a separate catalog entry, surfaced according to the same `InputType` filtering rules as existing entries.

### Modified Capabilities

*(none — this change adds entries to the catalog without changing when or how completions are triggered)*

## Impact

- `src/completions/jq_builtins.rs` — new catalog entries; one detail string fix
- `docs/maturity/` — files moved to `completed/`, README updated
- No changes to `json_context.rs`, `suggestions.rs`, or any other module
- No breaking changes; all existing tests continue to pass
