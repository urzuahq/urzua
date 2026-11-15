---
Stable-Id: 01M7FVTZWERYF4SQ5WEMK3990A
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 227 — Remove the deprecated audit alias

## What

Remove the `audit` command, whose removal the binary already announces.

## Why

the audit of 2026-11-15 found `audit` deprecated with removal announced and no record tracking the removal.

## Done means

- `urzua audit` is an unknown command, and the changelog names `check` as the replacement.
- No documentation page, hook or action calls `audit`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
