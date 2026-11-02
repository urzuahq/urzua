---
Stable-Id: 01M6CXZX537SRJ6YWNB7EGAM86
Status: Planned
Phase: '0'
Track: schema-governance
Blocked-on: MILE-123 (a diff-scoped run to evaluate against)
---
# 124 — A rule requiring that a diff changing declared source paths also changes a governing record

## What

A rule, declared in config, requiring that a diff which changes declared source paths also changes a
governing record. The paths and the record set are declared; the rule is evaluated against a diff and
reports a source change with no accompanying record change. It depends on a diff-scoped run
(`MILE-123`).

## Why

Replacing a hand-written linter outright (`MILE-102`) means every convention such a linter enforces
needs a declared home, and source-change-requires-record-change is a common one. It is the mechanical half of the
decision-before-implementation idea `MILE-14` leaves undecided: this rule asks only that a record
changed, never that it is the right one, per `AGENTS.md`'s limit on what `check` may claim. `MILE-14`
remains the place to decide whether such a gate should exist by default.

## References

- `MILE-14` -- decide RFC-12's decision-before-implementation gate.
- `MILE-123` -- the diff-scoped run.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Replacing a linter outright needs source-change-requires-record-change to have a declared home; filed for that. | **substantive** |
