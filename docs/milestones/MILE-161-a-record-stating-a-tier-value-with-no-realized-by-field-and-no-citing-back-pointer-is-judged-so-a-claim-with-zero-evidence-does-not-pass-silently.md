---
Stable-Id: 01M6JW44A9FTFNEAK19MVS7EN0
Status: Done
Phase: '0'
Track: embodiment-model
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/embodiment.rs, test:rust/crates/urzua-cli/tests/test_tier_integration.rs
---
# 161 — A record stating a tier value with no Realized-by field and no citing back-pointer is judged, so a claim with zero evidence does not pass silently

## What

Judge a record that states a tier value (Verified or Implemented) while carrying no Realized-by field and no back-pointer citing it, instead of skipping it.

## Why

`MILE-149` made embodiment consistency judge a record with no Realized-by field when back-pointers cite it, but a record with neither is still not examined, so a claim of Verified or Implemented with zero evidence passes silently. That breaks the no-silent-no-op principle: a rule that finds nothing must be distinguishable from one that ran on nothing. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A record stating Verified or Implemented with no evidence of either kind is reported as a finding; a record that states no tier stays unjudged and is counted as such; a fixture covers all three cases and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: `embodiment.consistency` reports a record stating `Verified` or `Implemented` with neither a `Realized-by` field nor a citing back-pointer, and a record stating no tier stays unexamined in the census; `test_tier_integration.rs` covers all three cases. Run over this repository it found three records claiming `Implemented` with no evidence (ADR-7, ADR-8, ADR-28), now given the `Realized-by` the work already named in their References. `urzua fix` is unchanged and does not repair these. `Status` stays `Planned`. **Why:** the Done-when contract. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: `embodiment.consistency` reports a stated `Verified` or `Implemented` with no evidence. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
