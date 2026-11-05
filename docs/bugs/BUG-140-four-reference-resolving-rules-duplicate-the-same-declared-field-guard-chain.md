---
Stable-Id: 01M373DYSJ4RC12J0XKH8EJRYG
Status: Fixed
Found-in: "MILE-112's duplication sweep"
Realized-by: test:rust/crates/urzua-core/src/rules/pointer.rs#a_non_resolving_pointer_is_an_error@sha256:b574f4ec3caea8639cd99fd8fcc722e86f999a23872cc99b9d2ece710adc6a9a, test:rust/crates/urzua-core/src/rules/pointer.rs#target_status_judges_only_the_statuses_a_repository_declared@sha256:3dec110571d2cb7ad4e48e74a906d9474641f8118fbeb3e030e833258a2a2046, test:rust/crates/urzua-core/src/rules/narrative.rs#blocked_on_free_text_with_no_reference_is_skipped@sha256:b0a65ebce1923dd13cbac83ed5d0e5dca3d384e37d3caca14cf1c1d347158cc8, test:rust/crates/urzua-core/src/rules/relation.rs#relation_target_status_undeclared_flags_a_resolving_reference_to_an_undeclaring_type_observed_failing@sha256:ca11ae7cf1c144e89c5a032cd910d68b79f39f8ff384b65143ce0f8d23a253c7
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
> | 2026-11-05 | `Regression-test` retired (ADR-84); the 4 existing tests that cover the change are now cited as `test:` locators in `Realized-by`, and the prose text is removed. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
