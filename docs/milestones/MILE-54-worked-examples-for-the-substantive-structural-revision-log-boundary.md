---
Status: Planned
Stable-Id: 01M1YN88XE8TM1YVHK24PR5WWM
Phase: '1'
Track: governance-process
Implements: RFC-6
Blocked-on: —
---
# 54 — Worked examples for the substantive/structural revision-log boundary

## What

Add worked examples (real entries from this repo's own revision logs, classified and explained) to the substantive/structural guidance, and consider whether the two-value enum needs a third state or clearer criteria.

## Why

`revision-log.change-class-required` already checks that a classification exists; nothing checks whether it's the *correct* one, and this project's own experience already shows the boundary is genuinely hard to call -- getting it wrong is exactly how a substantive change reads as structural-only after the fact.

## Done means

- The revision-log guidance in `SPEC-2` and `RFC-6` carries worked examples taken from this repository's own revision logs, each classified substantive or structural with the reason.
- The guidance records whether the two-value class needs a third state or sharper criteria (decided in an ADR if it changes).
- A worked example set is checked to name rows that exist.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
