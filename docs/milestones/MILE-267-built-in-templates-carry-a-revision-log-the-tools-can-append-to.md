---
Stable-Id: 01M7FW6543EQ20WQ97EB59PGA0
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 267 — Built-in templates carry a revision log the tools can append to

## What

Ship a revision log with a first row in every built-in template, and let `fix --apply` and `reverify --apply` append to an empty log table.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary. A record made by `urzua new` from the built-in decision template could not take `fix --apply` because it has no revision log, and adding an empty table failed for having no row to append after.

## Done means

- A record created by `urzua new` from each built-in profile accepts `fix --apply` with no hand edit, shown by a test per profile.
- An empty revision-log table receives its first row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
