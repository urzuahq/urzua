---
Stable-Id: 01M6D5VKQZHEFD36V5YQVV0CSG
Status: Fixed
Found-in: "A /code-review pass, round 30"
Regression-test: "rules::tests::a_dot_or_dotdot_spelled_locator_resolves_to_the_tracked_file_observed_failing"
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
