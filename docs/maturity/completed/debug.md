# `debug`

**Category:** Universal  
**Input type:** Any

## What's implemented

`InputType::Any` catalog entry. Insert text is bare `debug`. Detail string notes "jaq: no message argument".

jaq's `debug` passes the current value through and prints it to stderr — no message argument is supported. The catalog correctly reflects this.

## What's missing

Nothing for jaq. Standard jq supports `debug("message")` but jaq does not. The limitation is documented in the detail string.

## Estimated complexity

N/A — already correct for jaq.

## Needs changes to overall logic

No.
