---
Stable-Id: 01M6CXZY4QSHAD89X81W3MW6SN
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/rules/relation.rs, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs
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

A relation claim (`Supersedes` or `Superseded-by`) made by a record that is not yet accepted is reported as pending (a warning) and becomes an error once the claiming record is accepted, declared through the pair `provisional` option; `provisional_claim_integration.rs` covers a Draft claim, an Accepted claim with no reciprocation and a reciprocated claim.

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
