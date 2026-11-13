---
Status: Planned
Stable-Id: 01M1YN81YFRMVT2EWYM4PCKP5X
Phase: '1'
Track: schema-governance
Implements: RFC-1
Blocked-on: —
---
# 41 — Add a code-path binding field (affects) to the core schema

## What

Add an `affects` field to the core schema so a decision record can declare which code paths its outcome touches -- a path/glob list, forward-looking, distinct from `Realized-by`'s backward-looking evidence.

## Why

Every relationship the schema has today points backward: a record cites its evidence (`Realized-by`), or another record (`Implements`/`Derives-from`). Nothing lets a decision declare what it touches going forward, so there's no way to flag "this decision's affected path just changed, has anyone re-checked it still holds" -- exactly the kind of drift `embodiment.consistency` already catches for evidence, but for the decision's own stated scope of effect.

## Done means

- An `affects` field is declared in config as a list of paths and globs on a decision record.
- `check` reports a pattern that matches nothing.
- A record whose affected path changed since the record was last touched is reported by a rule distinct from `embodiment.consistency`.
- A fixture covers a changed path and an untouched one.
- The field is exercised on urzua's own ADRs.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
