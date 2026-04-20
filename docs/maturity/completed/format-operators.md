# Format operators: `@base64` / `@base64d` / `@uri` / `@html` / `@sh` / `@json` / `@text` / `@csv` / `@tsv`

**Category:** String (most) / ArrayOfScalars (`@csv`, `@tsv`)  
**Input type:** String for encoding operators; `array_scalars` for `@csv` / `@tsv`

## What's implemented

All format operators have type-aware catalog entries. They appear only when the appropriate input type flows into the pipe (`@base64` requires a string; `@csv`/`@tsv` require a flat array of scalars). No argument needed; insert text is the bare operator.

`@csv` and `@tsv` are jqpp extensions implemented in `executor.rs` — jaq does not provide them natively. Their detail strings note this.

## What's missing

Nothing. These are zero-argument operators; the type filter already prevents them from appearing at the wrong pipe stage.

## Estimated complexity

N/A — already complete.

## Needs changes to overall logic

No.
