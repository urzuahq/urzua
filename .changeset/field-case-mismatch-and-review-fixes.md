---
default: patch
---

Fixes two opposite-direction defects in how a declared field's value is read when its exact-cased key
is missing (`ADR-57`): `claim.status-agreement` and `pointer.target-status` used to fabricate a
sentinel that turned every claim or pointer citing a mis-cased `Status` field into a false blocking
finding; the embodiment rules used to silently drop a mis-cased `Realized-by`/`Embodiment`, hiding a
real inconsistency. Both now skip the field (unjudged, not judged-and-wrong), and a new rule,
`header.field-case-mismatch`, is the one place that reports a declared field written under a different
case (`ADR-58`). Also fixes a duplicate record-number bug in `urzua new` (`0013-01-15-slug.md` used to
parse as a date and skip record 13, which could then be reissued), consolidates three previously
independently-computed "record-shaped file" definitions into one, and consolidates the `(record,
field)` candidate-slot construction duplicated across five rules into two shared helpers. All changes
verified as no-ops against this repository's own corpus except the two fixed defects, which have
regression tests.
