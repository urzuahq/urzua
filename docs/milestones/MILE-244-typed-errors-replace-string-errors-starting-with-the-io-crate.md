---
Stable-Id: 01M7FVZ1GE73HXVKEPFW9BNKX2
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 244 — Typed errors replace string errors, starting with the IO crate

## What

Replace `Result<_, String>` with typed error enums, starting with the IO crate and then the command layer, so a caller can branch on the cause.

## Why

the audit of 2026-11-15 counted 232 functions returning a string error and 86 conversions to a string, with one failure type that is a string plus one optional cause.

## Done means

- The IO crate has no public function returning a string error.
- Each could-not-run cause in a report maps from a typed error, and messages are unchanged in the golden tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
