---
Stable-Id: 01M6D24G08YR10FW9QP3F28WMW
Status: Fixed
Found-in: "MILE-119, writing an all-rules header_shape: none test"
Regression-test: "every_header_reading_rule_reports_a_header_none_type_as_out_of_scope in urzua-cli/tests/check_integration.rs"
---
# 152 — nine more rules misclassify header-shape-none slots as unreadable or absent instead of out of scope

## What was wrong

Nine rules built a per-record or per-slot population with no `header_shape: none` gate (`ADR-50`), so
a record of such a type was counted `Unreadable` (or `Absent`, for `header.layout-consistency`)
instead of `OutOfScope`: `header.layout-consistency`, `pointer.resolution`, `pointer.target-status`,
`narrative-field.stale`, `embodiment.consistency`, `embodiment.locator-exists`,
`embodiment.locator-promotion-candidate`, `relation.supersession-reciprocity` and
`relation.target-status-undeclared`. A type with no header has nowhere for a header to fail to parse,
so the census reported a malformed corpus where the configuration simply declared none.

## Why nothing caught it

`BUG-137`, `BUG-142` and `BUG-147` each swept only the rules a review pass looked at, and gating was
written separately inside each rule. Nothing enumerated every shipped rule against a `none` type.

## Fix

`scoped_census_records` in `rules.rs` applies the classification once; every rule that reads a header
builds its population through it, and the four per-rule gates were removed. The all-rules test iterates
`ALL_RULES` and names each exemption (rules that never read a header). Observed failing on 13 rules
(these nine plus the four `BUG-147` rules) before the change, passing after.

## References

- `MILE-119`; `BUG-98`, `BUG-124`, `BUG-137`, `BUG-142`, `BUG-147`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed with `MILE-119`'s shared population builder. **Why:** the same `ADR-50` gating gap recurred in nine rules beyond the five already fixed. | **substantive** |
