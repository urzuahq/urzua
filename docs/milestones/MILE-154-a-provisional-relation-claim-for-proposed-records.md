---
Stable-Id: 01M6JKNSMZTNG2QVMMP4DPN9P1
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_the_peer_has_not_echoed_is_an_error@sha256:51ca8c61a9ff08a6df2928883e0fbc13b6de005fa9d3ac983a877c10885c69d2, code:.urzua/config.yaml, code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml
---
# 154 — A provisional relation claim for Proposed records

## What

Add a provisional-claim concept: a relation claim made by a Proposed record that the peer echoes only in a pending field is reported at a lower, declared severity than a settled claim.

## Why

A real adopter treats an unreciprocated claim from a Proposed record as a warning, but `relation.pair-reciprocity` is a single level and blind to status, so it errors. A claim not yet accepted is a different fact from a broken accepted claim, and one severity for both is either too loud or too quiet. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- `relation.pair-reciprocity` reports a Proposed-record claim echoed only in a pending field at the provisional severity. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95
- `relation.pair-reciprocity` reports an Accepted-record claim at the full severity. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_the_peer_has_not_echoed_is_an_error@sha256:51ca8c61a9ff08a6df2928883e0fbc13b6de005fa9d3ac983a877c10885c69d2
- The status split is declared in configuration. — evidence: code:.urzua/config.yaml, code:rust/crates/urzua-core/src/config.rs
- Fixtures cover both. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_the_peer_has_not_echoed_is_an_error@sha256:51ca8c61a9ff08a6df2928883e0fbc13b6de005fa9d3ac983a877c10885c69d2
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: a relation pair declares `provisional: {statuses, level}`; `relation.pair-reciprocity` reports an unechoed claim from a claimant at those statuses at the provisional level and any other at the rule level. This repository declares the `amendment` pair (RFC `Amends` / `Amended-by`) with `statuses: [Draft, Discussion]`; RFC-25 is the live provisional case, covered by a waiver that expires 2027-02-04. `Status` stays `Planned`. **Why:** the Done-when contract; status is a human decision. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: a relation pair declares `provisional: {statuses, level}`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
