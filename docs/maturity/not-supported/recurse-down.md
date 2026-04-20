# `recurse_down` — not supported

**jq behavior:** Alias for `recurse` — performs recursive descent through all nested values.

**Why not supported:** jaq 3.x does not define `recurse_down`. The canonical function is `recurse` (or `recurse(.[]?)`).

**Workaround:** Use `recurse` directly. No behavior difference.

## Could it be added?

Yes — `def recurse_down: recurse;` as a preamble. But since it's a pure alias with no added value, there is no practical reason to add it.

## Estimated complexity

`Low` — trivial alias definition.

## Needs changes to overall logic

Yes — requires the query preamble mechanism.
