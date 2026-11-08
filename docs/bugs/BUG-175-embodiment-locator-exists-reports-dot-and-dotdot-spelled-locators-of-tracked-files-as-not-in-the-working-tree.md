---
Stable-Id: 01M6D5VKQZHEFD36V5YQVV0CSG
Status: Fixed
Found-in: "A /code-review pass, round 30"
Realized-by: test:rust/crates/urzua-core/src/rules/embodiment.rs#a_dot_or_dotdot_spelled_locator_resolves_to_the_tracked_file_observed_failing@sha256:7d80cd1649a608eab2bfa47b8b2acce384b70ff649fc24d3522a2ac03c78b46d
---
# 175 — embodiment.locator-exists reports dot and dotdot spelled locators of tracked files as not in the working tree

## What was wrong

`embodiment.locator-exists` reported `Realized-by: code:./src/b.rs` and `code:src/../src/b.rs` as "not in
the working tree" although the file exists and is tracked, while `embodiment.consistency` resolved the same
spellings (git pathspecs normalize them) and computed drift for them. The two rules disagreed on whether
the locator was a real path, and the blocking finding asserted something false.

## Why nothing caught it

The existence check is exact membership in the tracked path set; drift detection hands the raw string to
git. Neither normalized, so only git's own tolerance hid the difference.

## Fix

`normalize_locator` (lexical: drops `.` and empty segments, resolves `x/..`) is applied once in
`parse_realized_by`, so locator-exists, drift detection and the promotion-candidate rule all read the same
path. Whether a directory locator is legal is `BUG-176`.

## References

`BUG-176`, `ADR-32`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed. **Why:** found by a round 30 review and reproduced against a disposable fixture. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs#a_dot_or_dotdot_spelled_locator_resolves_to_the_tracked_file_observed_failing` (lines 1294-1319) 743c032e1270 -> 7d80cd1649a6. Reason: Read the diff of each pinned span since the old pin and the claim still holds. embodiment.rs#a_dot_or_dotdot_spelled_locator_resolves_to_the_tracked_file_observed_failing: the call to the locator checker gained one more empty-slice argument, assertions unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
