---
Status: Planned
Stable-Id: 01M1YN835FM8NATNCKA34F6FED
Phase: '1'
Track: governance-process
Blocked-on: —
---
# 43 — Write-time reciprocity auto-linking when a record supersedes another

## What

When `urzua new`/`fix` writes a `Supersedes`/`Superseded-by` claim, write the reciprocal pointer on the target record automatically, instead of relying on a human to author both sides and `relation.supersession-reciprocity` to catch it after the fact.

## Why

`relation.supersession-reciprocity` already detects a one-directional supersession claim -- real, useful, but reactive. Auto-linking at write time prevents the defect from ever existing instead of catching it on the next `check` run, the same shift from detect to prevent this project already made for other write paths (`fix`'s gated apply mode, ADR-14's revision log).

## Done means

`urzua new` and `urzua fix --apply` writing a `Supersedes` or `Superseded-by` claim also write the reciprocal pointer on the target record, with a revision-log row on each, a second run is a no-op, and a claim whose target cannot be written is refused and reported; a fixture shows `relation.supersession-reciprocity` has nothing to report afterwards, and the behaviour is exercised in urzua's own repository.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |