---
Stable-Id: 01M6HZ0MQVAKV54TM17N8MY1HN
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
---
# 138 — A record not yet accepted cannot state Embodiment Implemented or Verified

## What

A cross-field rule: a record whose Status is not an accepted-type status cannot state an Embodiment of `Implemented` or `Verified`. The rule reads the declared status vocabulary from `MILE-132` and fails with an error naming both values.

## Why

`embodiment.consistency` compares the stated Embodiment with the tier computed from evidence and never reads Status, so a Proposed record can claim to be Verified and pass. The adopter's linter blocks that combination, since a decision not yet made cannot have been built. Without the check the claim goes unnoticed until the record is accepted.

## Done means

A planted record with a not-yet-accepted status and `Embodiment: Implemented` (and again with `Verified`) is an error, and the same record accepted is not; `Specified` and `Inactive` on a not-yet-accepted record are not flagged; the rule reports how many records it examined.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter blocks Implemented or Verified on a record that is not accepted, and urzua's embodiment rule never reads Status. | **substantive** |
