---
Stable-Id: 01M37DNSJAGG0031AJKFXAFCM2
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 27"
Regression-test: "a_placeholder_entry_mixed_with_a_real_reference_is_not_flagged_observed_failing, rules.rs"
---
# 148 — header.pointer-field-clean rejects the no-value placeholder mixed with a real reference

## What was wrong

`header_pointer_field_clean`'s own doc comment says explicitly: "every comma-separated entry, once
trimmed, is *exactly* a reference token (or the `—` no-value placeholder)" -- a per-entry claim. The
code only checked the placeholder against the *whole* field value before splitting on commas, then
checked each split entry only with `is_record_reference`, which returns `false` for `—`. A field
written as `Derives-from: RFC-1, —` (one real reference plus the documented placeholder) had its
whole-value check fail (the value isn't only `—`), split into `["RFC-1", "—"]`, and flagged `—` as
"isn't a clean reference" -- a spurious finding for exactly the input shape the rule's own doc comment
says must be accepted.

## Why nothing caught it

The three existing tests for this rule each cover one shape in isolation (a bare clean reference,
multiple clean references, a bare placeholder field) -- none covers the mixed case the doc comment
itself describes.

## Fix

The per-entry check now also accepts `crate::values::is_no_value_sentinel(entry)`, matching the doc
comment's own stated contract.

## References

- `values::is_no_value_sentinel` -- the shared predicate (`BUG-138`) this fix reuses rather than
  re-deriving.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-24 | Filed and fixed in one pass, found by a full-release code review. **Why:** the rule's own doc comment already specified per-entry placeholder acceptance; the implementation only checked the whole value. Planted-violation test observed failing before the fix, passing after. | **substantive** |
