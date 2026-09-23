---
Stable-Id: 01M36S90N3TVNV9YVV9K6GK8M3
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 24"
Regression-test: "not yet written -- Status: Open, performance-only, no correctness impact"
---
# 134 — declared_slots_for_roles and declared_cross_record_value rebuild a type's declared-field set per record instead of using the cached by_type map

## What was wrong

`RecordTypeConfig::declared_fields()` builds a fresh `BTreeSet` from `required_fields`/`known_fields`
on every call. `declared_fields_by_type` (`rules.rs`) exists specifically to compute this once per
type and cache it in a `HashMap`, and several rules already use that cache correctly
(`field_untrimmed_value`, `header_field_case_mismatch`).

`declared_slots_for_roles` (used by `relation.supersession-reciprocity`/`relation.target-status-
undeclared`) calls `t.declared_fields()` inside a `.filter()` closure that runs once per record, and
`declared_cross_record_value` (used by every cross-record `Status` read: `pointer.target-status`,
`narrative-field.stale`, `claim.status-agreement`) calls it once per resolved reference -- both
rebuilding a value that is actually constant per type, scaling as O(records) or O(records ×
references) instead of O(types) for a single `check` run.

## Why this is filed, not fixed here

Not a correctness bug -- findings and behavior are unaffected, only wasted CPU. Fixing it means
threading a precomputed `&HashMap<String, BTreeSet<FieldName>>` into both functions' signatures
(and their callers, several rules deep), which is a real but mechanical refactor better done as its
own reviewed change than appended to an already-large round.

## References

- `declared_fields_by_type` -- the existing cache this bug's fix would reuse.
- `field_untrimmed_value`/`header_field_case_mismatch` -- the two rules already doing this correctly,
  the pattern to match.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified both call sites rebuild a per-type-constant value inside per-record/per-reference loops instead of using the cache this codebase already has for exactly this. Performance-only, left `Status: Open` for its own change rather than rushed into an already-large round. | **substantive** |
