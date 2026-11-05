---
Stable-Id: 01M376A49FF9QM8YWCKQARWKW2
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 26"
Realized-by: test:rust/crates/urzua-core/src/rules/pointer.rs#target_status_judges_only_the_statuses_a_repository_declared@sha256:3dec110571d2cb7ad4e48e74a906d9474641f8118fbeb3e030e833258a2a2046, test:rust/crates/urzua-core/src/rules/relation.rs#relation_target_status_undeclared_flags_a_resolving_reference_to_an_undeclaring_type_observed_failing@sha256:ca11ae7cf1c144e89c5a032cd910d68b79f39f8ff384b65143ce0f8d23a253c7
---
# 144 — pointer.target-status and relation.target-status-undeclared duplicate the same resolved-target walk

## What was wrong

`pointer_target_status` and `relation_target_status_undeclared` each independently walked a declared
field's resolved references the same way -- resolve the field's references, look each up in the index,
skip a lookup miss, and name the field that plays the `Status` role for the target's type -- before
their own, genuinely differing judgment of the target (one checks the target's status against a
blocklist; the other checks whether the target's type declares a status field at all).

## Why nothing caught it

Each rule's own review compared its differing per-reference logic against its own history, not against
the setup code the other rule shares line-for-line.

## Fix

Extracted `for_each_resolved_status_target(record, field_name, config, index, body)`, taking the
differing judgment as a closure; both rules now supply only their own predicate. No behavior change:
full test suite and real-corpus finding count unchanged.

## References

- `BUG-140`/`BUG-141` -- the same duplication-drift shape found in the same review, at other places in
  this file.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass, found by a full-release code review. **Why:** two rules shared the identical resolved-target walk, differing only in their final predicate. No behavior change: full suite and real-corpus finding count unchanged. | **substantive** |
> | 2026-11-05 | `Regression-test` retired (ADR-84); the 2 existing tests that cover the change are now cited as `test:` locators in `Realized-by`, and the prose text is removed. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
