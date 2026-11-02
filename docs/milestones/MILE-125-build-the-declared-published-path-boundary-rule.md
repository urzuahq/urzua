---
Stable-Id: 01M6CXZXG5N2XF4J9AZ8TFGBXG
Status: Planned
Phase: '0'
Track: schema-governance
Implements: RFC-11
---
# 125 — Build the declared published-path boundary rule

## What

Build the declared published-path boundary rule `RFC-11` decided: config declares a set of paths that
publish beyond the repository, and a record identifier cited from inside one is a violation. Per
`RFC-11` section 1, a boundary is a declared fact, not a rule; section 2, severity is part of the
declaration; section 3, a violation and evidence are separate questions about the same text.

## Why

`RFC-11` is Accepted and its boundary mechanism is unbuilt. A repository that publishes a subset of
its tree keeps internal references out of it by convention only, which is the failure `RFC-11` exists
to remove, and an adopter replacing a hand-written linter outright (`MILE-102`) has to be able to
delete whatever script enforces the boundary today.

## References

- `RFC-11` -- declared boundaries; sections 1 to 3.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `RFC-11` (Accepted) decided declared published-path boundaries and nothing builds them. | **substantive** |
