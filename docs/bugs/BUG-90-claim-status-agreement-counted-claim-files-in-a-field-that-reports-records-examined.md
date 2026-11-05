---
Stable-Id: 01M2YR3J3H6MGCCA71FXNGZC5V
Status: Fixed
Found-in: "Round 10 of the 0.4.0 review, reproduced against a scratch repository"
Realized-by: test:rust/crates/urzua-core/src/rules/claim.rs#several_claim_files_closing_one_record_count_as_one_claim_observed_failing@sha256:523633f34e52e6fa832ab64b80cdf343d4617742fd787f53046f310e8815f3f4
---
# 90 — claim.status-agreement counted claim files in a field that reports records examined

## What was wrong

`claim_status_agreement` incremented `records_examined` once per claim *file*. In a corpus of one
record with three reachable claim files it reported `records_examined: 3, scope: records` -- a count
larger than the corpus, in the field whose whole purpose is to say how many records a rule examined.

`BUG-81` and `BUG-83` added `RuleScope::Config` and `RuleScope::Paths` for exactly this: a rule whose
count is not records must say so. This rule's count was neither -- it does resolve claims against the
record index, so its scope is right and its counter was wrong.

Now counted per distinct record a claim resolved to.

## Why nothing caught it

`records_examined` was only ever read by humans until `ADR-55` made it load-bearing two days ago.
`BUG-78`, `BUG-81`, `BUG-83` and this are four separate discoveries that the number was untrue, all
within those two days, each found by something newly depending on it.

Nothing asserts the obvious invariant: a record-scoped rule cannot examine more records than the
corpus contains.

## References

- `BUG-78`, `BUG-81`, `BUG-83` -- the same field untrue for other reasons.
- `ADR-55`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Regression test added: `several_claim_files_closing_one_record_count_as_one_claim_observed_failing`, observed failing with the fix undone and passing with it. **Why:** this record named no test, so its `Fixed` had no evidence behind it under ADR-84. | **substantive** |
> | 2026-11-05 | `Regression-test` retired (ADR-84); its value named no test, so a regression test was written and cited in `Realized-by`. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
