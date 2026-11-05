---
Stable-Id: 01M371W49G54XZ0GQA1BR9XPYH
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 25"
Realized-by: test:rust/crates/urzua-core/src/rules/pointer.rs#a_bare_placeholder_field_is_still_not_flagged@sha256:2f29423cd4dacbc53e2e2bb75a43edcbd8203b59289cec94f3153e2271c7b9db, test:rust/crates/urzua-core/src/rules/pointer.rs#a_placeholder_entry_mixed_with_a_real_reference_is_not_flagged_observed_failing@sha256:18333959388ee7c8a9ccb8c4003c1e740723ca98149bfb254299fa53ab40f3fd, test:rust/crates/urzua-core/src/rules/relation.rs#an_em_dash_supersession_value_answers_its_slot@sha256:63eacb42bcff10fc5ac7b9c1062d58361267e302ff11be0b4e2a2a562c6185c3
---
# 138 — the no-value em-dash sentinel is duplicated in two rules instead of shared

## What was wrong

The em-dash "no value" sentinel check (`value.trim() == "—"`, this corpus's written way of answering a
pointer/relation field with "nothing here") was re-implemented byte-for-byte identically in two rule
functions (`header_pointer_field_clean` and `supersession_reciprocity`), with no shared predicate in
`values.rs` — the module that otherwise centralizes exactly this kind of domain primitive (`RecordId`,
`FieldName`).

`SPEC-22`'s duplication rule extracts on the second occurrence, not the third; this was already past
that threshold. Not yet a live divergence (both copies agreed), but the exact shape `BUG-111`/`BUG-114`
show this project has already hit twice: a third caller needing the same distinction would have to
re-copy the literal Unicode character and comparison, and a typo or omission there silently miscounts
an answered-empty field as `Absent`.

## Why nothing caught it

Each rule was written independently and reviewed on its own merits; nothing cross-checks that a
constant used in two places stays defined in one.

## References

- `values.rs`'s existing `RecordId`/`FieldName` newtypes — the established home for this kind of shared
  domain primitive.
- `SPEC-22` — the duplication-on-second-occurrence rule this was already past.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review. Extracted `values::is_no_value_sentinel`; both call sites now share it. No behavior change -- both existing call sites' own tests cover the em-dash case unchanged. | **substantive** |
> | 2026-11-05 | `Regression-test` retired (ADR-84); the 3 existing tests that cover the change are now cited as `test:` locators in `Realized-by`, and the prose text is removed. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
