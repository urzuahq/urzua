---
Stable-Id: 01M6NKH2K59EAX2PJWXXK32A1H
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-54-one-stray-record-shaped-file-collapses-every-sibling-record-type-into-its-parent.md
Reason: "Its regression test, `a_container_directory_does_not_absorb_the_types_beneath_it_observed_failing`, exercised the container-directory rule in `init`'s inference. `RFC-35` replaced that inference with a declared `dir`, so the rule and its test were deleted and this record's revision log marks it superseded by `RFC-35`. A test of the original behaviour can no longer exist."
Category: test-removed-by-decision
Justified-by: RFC-35
---
# WAIVER-0048 — BUG-54 needs no test: its test was deleted when RFC-35 replaced the heuristic

## What this covers

`status.delivery-agreement` reports `BUG-54`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Its regression test, `a_container_directory_does_not_absorb_the_types_beneath_it_observed_failing`, exercised the container-directory rule in `init`'s inference. `RFC-35` replaced that inference with a declared `dir`, so the rule and its test were deleted and this record's revision log marks it superseded by `RFC-35`. A test of the original behaviour can no longer exist.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `test-removed-by-decision` with a citation of `RFC-35`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-54-one-stray-record-shaped-file-collapses-every-sibling-record-type-into-its-parent.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`. `RFC-35` — the decision that removed the test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
