---
Stable-Id: 01M6HZ0MQVAKV54TM17N8MY1HN
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Realized-by: code:rust/crates/urzua-core/src/rules/embodiment.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
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
> | 2026-11-04 | Built as the opt-in rule `embodiment.requires-accepted-status`. Accepted means the type's declared `statuses.success` (`MILE-132`); a type declaring none uses the rule's `accepted_statuses` option, and with neither it is not examined, the narrowest reading. **Why:** the rule needs a declared meaning of accepted rather than a guess from status names. | **substantive** |
> | 2026-11-04 | The rule also allows the claim at the type's declared `statuses.claim_retained` (`SPEC-3`); this repository's ADR type declares `[Superseded]`. **Why:** maintainer decision of 2026-11-04: a record may keep an Implemented or Verified claim in a status reached only after acceptance. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #178: opt-in rule `embodiment.requires-accepted-status`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
