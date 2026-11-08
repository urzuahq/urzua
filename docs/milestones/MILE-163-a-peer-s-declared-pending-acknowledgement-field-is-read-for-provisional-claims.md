---
Stable-Id: 01M6JW45AMX9BBJY1VQHP51ZH2
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089, code:rust/crates/urzua-core/src/config.rs@sha256:18818ee3b2262e4f5dd6b5960d7f874a5662afaabf772f15c82441d7cc7c2df0, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_acknowledges_is_not_reported_observed_failing@sha256:a77bb418d70bf5ecfbb56eee57f2921641960d983f8d8e3eb6c42c694f82ad06, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml@sha256:85f570143af61e013f36cc608d178c2724fec7d7903897e048eedeffdcb2e9af
---
# 163 — A peer's declared pending-acknowledgement field is read for provisional claims

## What

Read a declared field on the peer record that acknowledges a pending claim, so a provisional relation claim (`MILE-154`) is satisfied by the peer's own declaration as well as by the inverse relation.

## Why

A real adopter corpus records the acknowledgement on the peer under a field of its own naming. Without reading it, a provisional claim is reported as unacknowledged even though the peer states it. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- The field name is declared in configuration. — evidence: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089, code:rust/crates/urzua-core/src/config.rs@sha256:18818ee3b2262e4f5dd6b5960d7f874a5662afaabf772f15c82441d7cc7c2df0
- A provisional claim whose peer carries the declared acknowledgement is not reported. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_acknowledges_is_not_reported_observed_failing@sha256:a77bb418d70bf5ecfbb56eee57f2921641960d983f8d8e3eb6c42c694f82ad06
- A provisional claim whose peer does not carry the acknowledgement still is reported. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95
- A fixture covers both. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_acknowledges_is_not_reported_observed_failing@sha256:a77bb418d70bf5ecfbb56eee57f2921641960d983f8d8e3eb6c42c694f82ad06, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml@sha256:85f570143af61e013f36cc608d178c2724fec7d7903897e048eedeffdcb2e9af

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: a pair's `provisional.acknowledged_by` names the peer's acknowledgement field, read only for a provisional claim (`SPEC-3` 0.41, `SPEC-2` 0.53). Exercised here: the `amendment` pair declares `Amended-by-pending`, RFC-2 acknowledges RFC-25 in it, and the standing waiver for that provisional warning is deleted. **Why:** the milestone's Done means. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: a pair's `provisional.acknowledged_by` field settles a provisional claim. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-core/src/config.rs@sha256:18818ee3b226, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml@sha256:85f570143af6 Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml still declares test_paths, the statuses.pending pattern entries, the pending acknowledgement field behind provisional claims, the index format placeholders, drift_audit_field, the revision-log marker, waiver.matches-nothing and pending_until_required that these milestones dogfood (MILE-159 to MILE-169), commands/fix.rs still takes paths, proposed content and apply without a log (MILE-159), back_pointer.rs still matches test_paths as globs (MILE-160), config.rs still reads the declared pending-acknowledgement field (MILE-163), and the corpus-zero embodiment, relation, index, revision-log, waiver and field expectation files still exist and run this repository's own config. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
