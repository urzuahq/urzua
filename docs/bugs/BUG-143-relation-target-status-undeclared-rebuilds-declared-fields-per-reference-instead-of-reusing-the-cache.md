---
Stable-Id: 01M376A3SCJT064JEERV4CJ0T8
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 26"
Regression-test: "none -- performance-only, no behavior change; full suite and real-corpus finding count unchanged"
---
# 143 — relation.target-status-undeclared rebuilds declared-fields per reference instead of reusing the cache

## What was wrong

`relation_target_status_undeclared` called `t.declared_fields()` fresh inside its per-reference loop --
the exact `BUG-134` shape, in a rule that fix's own sweep did not touch. `declared_fields()` allocates a
new `BTreeSet` and constructs new `FieldName` values on every call, scaling as O(records × references)
instead of the O(types) every sibling rule already achieves via `declared_fields_by_type`.

## Why nothing caught it

`BUG-134` fixed `declared_slots_for_roles` and `declared_cross_record_value`'s three call sites
(`pointer_target_status`, `narrative_field_stale`, `claim_status_agreement`). `relation_target_status_undeclared`
reads a target's declared fields the same way but through its own inline check rather than
`declared_cross_record_value`, so it wasn't among the call sites that fix touched.

## Fix

Precomputes `declared_fields_by_type(config)` once, matching the pattern every sibling rule already
uses. No behavior change: full test suite and real-corpus finding count (70) unchanged.

## References

- `BUG-134` -- the same defect, first instance, in the other reference-resolving rules.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass, found by a full-release code review. **Why:** a missed sibling instance of `BUG-134`'s own defect class, in a rule that fix's sweep did not cover. Performance-only: full suite and real-corpus finding count unchanged. | **substantive** |
