---
Stable-Id: 01M36S90N3TVNV9YVV9K6GK8M3
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 24"
Regression-test: "none -- performance-only, no behavior change; the existing full suite (unchanged pass, unchanged 70-finding real-corpus check) is the fix's own evidence"
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

## Fix

`declared_slots_for_roles` is private and called only within `rules.rs`, so its fix is entirely
internal: it now computes `declared_fields_by_type(config)` once, before its `.filter()` over records,
instead of calling `t.declared_fields()` per record -- no signature change, no call sites touched.

`declared_cross_record_value` is called once per resolved reference (up to O(records × references)),
so caching inside it would not help; the cache has to live in its caller instead. Its signature changed
from `config: &Config` to `declared_by_type: &HashMap<String, BTreeSet<FieldName>>`, and its three
private call sites (`pointer_target_status`, `narrative_field_stale`, `claim_status_agreement`)
each compute the map once at the top of the rule, matching the existing pattern those same functions
already use for `pointer_fields_by_type`/`narrative_fields_by_type`.

No behavior change: the full test suite passes unchanged, and the real corpus reports the same 70
findings. No new regression test -- there is no wrong output to plant and observe; the unchanged
suite and finding count are the fix's own evidence that the cache lookup and the direct call agree.

## References

- `declared_fields_by_type` -- the existing cache this bug's fix would reuse.
- `field_untrimmed_value`/`header_field_case_mismatch` -- the two rules already doing this correctly,
  the pattern to match.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified both call sites rebuild a per-type-constant value inside per-record/per-reference loops instead of using the cache this codebase already has for exactly this. Performance-only, left `Status: Open` for its own change rather than rushed into an already-large round. | **substantive** |
> | 2026-09-23 | Fixed. `declared_slots_for_roles` fixed internally (private, no call sites touched). `declared_cross_record_value`'s signature changed to take the precomputed map; its 3 call sites now build it once per rule invocation, matching the pattern the same functions already use for other per-type maps. No behavior change: full suite and real-corpus finding count unchanged. | **substantive** |
