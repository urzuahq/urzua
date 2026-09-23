---
default: patch
---

Round 26's `/code-review` found and fixed three more defects, and filed one design-decision item:

- `field.quality`, `field.pending`, and `header.required-fields`'s own per-slot population loop
  misclassified a `header_shape: none` type's declared slots as `Unreadable` instead of `OutOfScope` --
  the same defect `BUG-137` fixed in `migrate::schema_report`, recurring in three more places (`BUG-142`).
- `relation.target-status-undeclared` rebuilt a type's declared-field set from scratch per reference
  instead of reusing the per-type cache every sibling rule already uses -- a missed sibling of `BUG-134`
  (`BUG-143`).
- `pointer.target-status` and `relation.target-status-undeclared` duplicated the same resolved-target
  walk; extracted into one shared helper (`BUG-144`).

All three: no behavior change, full test suite and real-corpus finding count unchanged.

Filed, not fixed: two hand-maintained rule-id allowlists carry the same drift risk the project's
`RuleOption` table was built to eliminate, but unifying them needs a real schema decision (`BUG-145`).
