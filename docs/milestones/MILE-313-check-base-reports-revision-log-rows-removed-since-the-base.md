---
Stable-Id: 01M7K8KEH2X3S1MCMZN0ZMR1W4
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 313 — check --base reports revision-log rows removed since the base

## What

Add a diff rule that reports, under `check --base`, every revision-log row a record had at the base that is absent at the head.

## Why

A revision log is the record's history, and nothing compares it between base and head. A review on 2026-11-16 showed a log holding a substantive row deleted by hand on a branch, the record renamed, and a tool-written log created in its place, with `check --base` clean. Refusing in the write commands is best effort; only a rule that compares the two sides is a guarantee.

## Done means

- Under `check --base`, a record whose log at the base has a row that is not present at the head produces a finding naming the row's date and class, shown by a test.
- The record is matched across a rename and across deletion and re-creation at a new path by the identity the configuration declares, shown by a test of each.
- A log section removed entirely is reported once for the section, not once per row.
- The rule is on by default at error, can be suppressed by the ordinary suppression record, and knows no record type: it reads only the configured log shape.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Found on 2026-11-16 by an independent review of a pull request; the maintainer said that day that friction and gaps met while using the tool are improvement points to file and fix. | **structural** |
