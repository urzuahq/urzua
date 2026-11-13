---
Stable-Id: 01M6CXZY4QSHAD89X81W3MW6SN
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/rules/relation.rs#provisional_severity@sha256:195be2597801efe99e476643c38c1c349a9daa112bf8040d29cb21cadf4cb8b1, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_the_peer_has_not_echoed_is_an_error@sha256:51ca8c61a9ff08a6df2928883e0fbc13b6de005fa9d3ac983a877c10885c69d2, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_is_not_settled_by_an_acknowledgement_observed_failing@sha256:fc20f0e8e7718fc9884f0d5dc3b447dea37a6d28632fa36566b0b80a9ea38b2e, code:rust/crates/urzua-core/src/config.rs#RelationPair@sha256:c4b384a54123ab4436c1d81d1f8d414ba78fbb056296c4ac3f1b9c1a2841fe03, spec:docs/specs/SPEC-3-configuration.md, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_echoed_draft_amendment_is_not_reported@sha256:7b21900df33133a5f558922d3995aca814771882defb8ff67a102bbf366632ab
---
# 127 — Decide whether a relation claim on a not-yet-accepted record is held to a weaker standard than an accepted record's

## What

Decide whether a relation claim on a record that is not yet accepted (a `Supersedes` or `Superseded-by`
on a `Draft` or `Proposed` record) is held to a weaker standard than the same claim on an accepted
record: a provisional, pending channel in which the claim is recorded and visible but does not yet
assert that the target is superseded.

## Why

`AGENTS.md` says a `Supersedes / Superseded-by` value asserts a decision was reached, and is therefore
not the author's to set. But a proposal legitimately intends to supersede something before it is
accepted, and a corpus that cannot say so either leaves the intent in prose or makes a premature
assertion. Today `relation.supersession-reciprocity` treats both sides identically. An adopter
corpus will have in-flight proposals that name what they would supersede, so the answer decides whether
its records can be checked without lying about their state.

## Decision (2026-11-04)

A relation claim from a record that is not yet accepted is reported as pending (a warning), and becomes an error once the claiming record is accepted.

## Done means

- A relation claim made by a record that is not yet accepted is reported as pending, a warning. — evidence: code:rust/crates/urzua-core/src/rules/relation.rs#provisional_severity@sha256:195be2597801efe99e476643c38c1c349a9daa112bf8040d29cb21cadf4cb8b1, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95
- The claim becomes an error once the claiming record is accepted. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_the_peer_has_not_echoed_is_an_error@sha256:51ca8c61a9ff08a6df2928883e0fbc13b6de005fa9d3ac983a877c10885c69d2, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_is_not_settled_by_an_acknowledgement_observed_failing@sha256:fc20f0e8e7718fc9884f0d5dc3b447dea37a6d28632fa36566b0b80a9ea38b2e
- The split is declared through the pair's `provisional` option. — evidence: code:rust/crates/urzua-core/src/config.rs#RelationPair@sha256:c4b384a54123ab4436c1d81d1f8d414ba78fbb056296c4ac3f1b9c1a2841fe03, spec:docs/specs/SPEC-3-configuration.md
- `provisional_claim_integration.rs` covers a Draft claim, an Accepted claim with no reciprocation and a reciprocated claim. — evidence: test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#a_draft_amendment_the_peer_has_not_echoed_is_a_warning@sha256:c80f77ab9888e7d35398361fa79e8dff356e5f66f60ea067d930bf5d7692ec95, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_the_peer_has_not_echoed_is_an_error@sha256:51ca8c61a9ff08a6df2928883e0fbc13b6de005fa9d3ac983a877c10885c69d2, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_echoed_draft_amendment_is_not_reported@sha256:7b21900df33133a5f558922d3995aca814771882defb8ff67a102bbf366632ab

## References

- `relation.supersession-reciprocity` -- the rule that treats both sides identically today.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Whether an in-flight proposal may name what it would supersede, without asserting it, is undecided and an adopter will hit it. | **substantive** |
> | 2026-11-04 | Decision recorded: a relation claim from a record that is not yet accepted is reported as pending (a warning) and becomes an error once the claiming record is accepted. `Status` stays `Planned`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: the decision is recorded (a claim from a not-yet-accepted record is pending, then an error once accepted) and built as the pair `provisional` option. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
