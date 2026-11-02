---
Stable-Id: 01M37DNS1DAPVN96J51G7RW6DR
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 27"
Regression-test: "four planted-violation tests in rules.rs, one per affected rule -- see References"
---
# 147 — four more rules misclassify header-shape-none slots as unreadable instead of out of scope

## What was wrong

`header_field_set_consistency`, `header_pointer_field_clean`, `field_untrimmed_value`, and
`header_field_case_mismatch` each check `record.header.is_unreadable()` with no gate for whether the
record's type declares `header_shape: none` (`ADR-50`) first -- the same defect class `BUG-137` fixed in
`migrate::schema_report` and `BUG-142` fixed in three other rules, recurring in four more places.

## Why nothing caught it

`BUG-142`'s own sweep was scoped to the three rules sharing `field_slots`-built required-field
populations (`field.quality`, `field.pending`, `header.required-fields`). These four rules build their
populations three other ways (`header.field-set-consistency` from `known_fields_by_type`;
`header.pointer-field-clean` from `fields_with_capability` + `field_slots`; `field.untrimmed-value` and
`header.field-case-mismatch` from `declared_fields_by_type` + `field_slots`), so they weren't in that
sweep's scope, and nothing then checked whether the same gating gap recurred in the rest of the file.

## Fix

Each of the four now gates on `has_no_header()` before `is_unreadable()`, returning `Outcome::OutOfScope`
-- matching `BUG-137`/`BUG-142`'s established pattern exactly.

## References

- `BUG-137`, `BUG-142` -- the same defect, in `migrate::schema_report` and three other rules.
- Four regression tests in `rules.rs`, one per rule:
  `a_header_none_type_is_out_of_scope_not_unreadable_for_header_field_set_consistency_observed_failing`,
  `a_header_none_type_is_out_of_scope_not_unreadable_for_header_pointer_field_clean_observed_failing`,
  `a_header_none_type_is_out_of_scope_not_unreadable_for_field_untrimmed_value_observed_failing`,
  `a_header_none_type_is_out_of_scope_not_unreadable_for_header_field_case_mismatch_observed_failing`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-24 | Filed and fixed in one pass, found by a full-release code review. **Why:** the same `ADR-50` gating gap `BUG-137`/`BUG-142` fixed recurred in four more rules their own sweeps did not cover. Four planted-violation tests observed failing before the fix, passing after. | **substantive** |
