# `test`

**Category:** String  
**Input type:** String

## What's implemented

Type-aware catalog entry. Insert text `test("")` positions the cursor inside the quotes for immediate typing. String-param completion is intentionally excluded for `test` — it takes a regex, not a data value.

## What's missing

Regex-aware argument suggestions. In theory, common regex patterns could be extracted from the actual string values in the live JSON (e.g., inferring a date pattern from `"2026-04-19"` and suggesting `"\\d{4}-\\d{2}-\\d{2}"`). In practice this is highly speculative and offers little value.

A simpler win would be suggesting **literal substrings** from the data (reusing the `FullString` extraction strategy already used by `contains`/`index`), since many `test` calls use simple literal matches rather than regex. This would be a low-noise improvement.

## Estimated complexity

`Medium` — needs a new strategy variant that offers literal-value suggestions inside regex-param functions without confusing users into treating them as regexes.

## Needs changes to overall logic

Yes — would require adding `test` to a new `LiteralString` param strategy group, or gating a weaker variant of `FullString` on the function name.
