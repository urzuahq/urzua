---
Stable-Id: 01M6JW45AMX9BBJY1VQHP51ZH2
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:.urzua/config.yaml, code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_acknowledges_is_not_reported_observed_failing@sha256:a77bb418d70bf5ecfbb56eee57f2921641960d983f8d8e3eb6c42c694f82ad06, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml
---
# 163 — A peer's declared pending-acknowledgement field is read for provisional claims

## What

Read a declared field on the peer record that acknowledges a pending claim, so a provisional relation claim (`MILE-154`) is satisfied by the peer's own declaration as well as by the inverse relation.

## Why

A real adopter corpus records the acknowledgement on the peer under a field of its own naming. Without reading it, a provisional claim is reported as unacknowledged even though the peer states it. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- The field name is declared in configuration. — evidence: code:.urzua/config.yaml, code:rust/crates/urzua-core/src/config.rs
- A provisional claim whose peer carries the declared acknowledgement is not reported. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_acknowledges_is_not_reported_observed_failing@sha256:a77bb418d70bf5ecfbb56eee57f2921641960d983f8d8e3eb6c42c694f82ad06
- A provisional claim whose peer does not carry the acknowledgement still is reported. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95
- A fixture covers both. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_acknowledges_is_not_reported_observed_failing@sha256:a77bb418d70bf5ecfbb56eee57f2921641960d983f8d8e3eb6c42c694f82ad06, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: a pair's `provisional.acknowledged_by` names the peer's acknowledgement field, read only for a provisional claim (`SPEC-3` 0.41, `SPEC-2` 0.53). Exercised here: the `amendment` pair declares `Amended-by-pending`, RFC-2 acknowledges RFC-25 in it, and the standing waiver for that provisional warning is deleted. **Why:** the milestone's Done means. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: a pair's `provisional.acknowledged_by` field settles a provisional claim. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
