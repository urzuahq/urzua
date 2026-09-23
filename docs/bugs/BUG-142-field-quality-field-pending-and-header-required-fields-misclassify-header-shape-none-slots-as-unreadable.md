---
Stable-Id: 01M376A35AT56F862RWJKPXTH3
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 26"
Regression-test: "three planted-violation tests in rules.rs, one per affected rule -- see References"
---
# 142 — field.quality, field.pending, and header.required-fields misclassify header-shape-none slots as unreadable

## What was wrong

`field_quality`, `field_pending`, and `header_required_fields`'s own per-slot population loop (distinct
from its record-scoped loop, which already gated correctly) each check `record.header.is_unreadable()`
with no gate for whether the record's type declares `header_shape: none` (`ADR-50`) first -- the same
defect class `BUG-137` fixed in `migrate::schema_report`, recurring in three more places that share the
same `field_slots`-built population.

The config schema allows a type to declare `header_shape: none` alongside a non-empty `required_fields`
list at the same time -- a self-contradiction only the *opt-in* lint `config.header-none-has-no-required-fields`
catches. Absent that lint, such a type's declared slots are always classified `Unreadable` by these
three rules (since a `none`-shaped record's `region` is always absent by construction), reporting a
parse failure that isn't one.

## Why nothing caught it

`BUG-137`'s fix was scoped to `migrate::schema_report` alone; nothing then checked whether the same
`field_slots`/`is_unreadable()` pattern recurred elsewhere. It does, in the three rules that share
`field_slots` for a required-field population.

## Fix

Each of the three now gates on `has_no_header()` before `is_unreadable()`, returning `Outcome::OutOfScope`
for such a slot -- matching `BUG-137`'s established pattern exactly. `header_required_fields`'s
record-scoped loop was already correct; only its separate per-slot population loop needed the gate.

## References

- `BUG-137` -- the same defect, first instance, in `migrate::schema_report`.
- `config.header-none-has-no-required-fields` -- the opt-in lint that catches the config contradiction
  this bug's own three rules could not tell apart from a genuine parse failure.
- The three regression tests, `rules.rs`: `a_header_none_type_is_out_of_scope_not_unreadable_for_field_quality_observed_failing`,
  `a_header_none_type_is_out_of_scope_not_unreadable_for_field_pending_observed_failing`, and
  `a_header_none_type_is_out_of_scope_not_unreadable_for_header_required_fields_observed_failing`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass, found by a full-release code review. **Why:** the same `ADR-50` gating gap `BUG-137` fixed in one place recurred in three more. Three planted-violation tests observed failing before the fix, passing after. | **substantive** |
