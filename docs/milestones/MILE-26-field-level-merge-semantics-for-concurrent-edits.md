---
Status: Planned
Stable-Id: 01M1Y5JT01RV9FEM7BWSX2VX2G
Phase: '3'
Track: review-workflow
Implements: —
Depends-on: MILE-79
Blocked-on: —
---
# 26 — Field-level merge semantics for concurrent edits

## What

Prevent two agents from clobbering different fields of the same record, rather than only detecting the collision after the fact.

## Why

`ADR-3` settles two agents creating different records at once and leaves two agents editing the same record open, which is the core mechanism gap; no RFC yet.

## Done means

- Two writers editing different fields of one record do not clobber each other: a concurrent edit merges field by field.
- A conflict on the same field is reported rather than overwritten.
- A test with two interleaved writers fails if either edit is lost.
- The guard is used by `urzua fix --apply` and the guarded edit command.
- The guard is exercised in urzua's own repository.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-79` added. **Why:** Done means says the field-level merge guard is used by `urzua fix --apply` and the guarded edit command, which `MILE-79` builds. Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Removed an appeal to research a reader cannot open; the argument now rests on `ADR-3`, which settles concurrent creation only. `Status` unchanged. | **structural** |
