---
Stable-Id: 01M373DYSJ4RC12J0XKH8EJRYG
Status: Fixed
Found-in: "MILE-112's duplication sweep"
Regression-test: "none -- pure extraction, no behavior change; each of the 4 rules' own existing tests cover the unreadable/absent/empty-references cases unchanged"
---
# 140 — four reference-resolving rules duplicate the same declared-field guard chain

## What was wrong

`pointer_resolution`, `pointer_target_status`, `narrative_field_stale`, and
`relation_target_status_undeclared` (`rules.rs`) each opened their per-slot census closure with the
identical guard chain, byte-for-byte, before a per-reference loop that differs:

```rust
if record.header.is_unreadable() {
    return Outcome::Unreadable;
}
let Some(value) = record.header.get(field_name.as_str()) else {
    return Outcome::Absent;
};
let references = extract_references(value);
if references.is_empty() {
    return Outcome::Absent;
}
```

Four independent copies of "resolve a declared field's raw references, or the `Outcome` its absence
means" -- past `SPEC-22`'s second-occurrence threshold by two.

## Why nothing caught it

Each rule was written and reviewed against its own per-reference loop, which does differ; the shared
setup above it was never singled out because no single rule's own review compared it against the
other three.

## Fix

Extracted `declared_field_references(record, field_name) -> Result<Vec<String>, Outcome>`; all four call
sites now open with `let references = match declared_field_references(record, field_name.as_str()) {
Ok(refs) => refs, Err(outcome) => return outcome };` and keep their own differing loop body unchanged.
No behavior change: the full test suite passes unchanged, and the real corpus reports the same 70
findings.

## References

- `BUG-133`/`BUG-134`/`BUG-135`/`BUG-138` -- the same duplication-drift shape found earlier in the same
  sweep, at other places in this file.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass, found by MILE-112's duplication sweep. **Why:** four rules independently re-derived the identical declared-field resolution guard chain. No behavior change: full suite and real-corpus finding count unchanged. | **substantive** |
