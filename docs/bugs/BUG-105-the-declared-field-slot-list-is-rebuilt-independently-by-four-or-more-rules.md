---
Stable-Id: 01M33QPNHQ85H3DKFRXKJYFRPJ
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "existing rule test suites, unchanged, exercised against the extracted helpers -- field_slots and pointer_and_narrative_slots, rules.rs"
---
# 105 — the declared field slot list is rebuilt independently by four or more rules

## What was wrong

The identical `(record, field)` candidate-slot construction — filter the corpus to records whose type
declares the field set, then flat-map each record against its declared fields — was written out in
full in `header_required_fields`, `field_pending`, `field_quality`, `field_untrimmed_value` (and, once
added, `header_field_case_mismatch`), each against a `HashMap` of either `Vec<String>` or
`HashSet<String>`. Separately, `pointer_target_status` duplicated `pointer_resolution`'s own slot
construction (pointer fields chained with narrative fields) statement-for-statement.

Beyond the repeated `O(records × fields)` traversal per `check` run, "which slots a rule governs" lived
in six places. A change to the derivation — a new field kind, a different declared-field source —
would need to be applied at each site by hand, with nothing to catch a site left behind. This is the
same class of risk `build_normalized_index` and `declared_slots` were already extracted to close for
their own shapes; this was the shape left over.

## Why nothing caught it

Each function's slot-building block was three to seven lines, looked self-contained, and was correct
on its own. No test compares the slot lists two rules build from the same inputs, so a divergence would
only be caught the way earlier drift was caught in this family: two rules disagreeing about one field
on one record, found by a reviewer reading the diff.

## What changed

Two shared functions replace the six inline constructions: `field_slots` (generic over
`HashMap<String, Vec<String>>` and `HashMap<String, HashSet<String>>` via a `where &'a C: IntoIterator`
bound) for the four single-field-source rules, and `pointer_and_narrative_slots` for the
pointer/narrative pair `pointer_resolution` and `pointer_target_status` both need. No rule's behavior
changed — verified via the existing test suites, unchanged, and a real-corpus `check` run reporting the
same 70 findings before and after.

## References

- `RFC-39` — names this class of drift for value comparisons and config projections; this is the same
  class for a derived candidate list.
- `BUG-106` — the sibling duplication (a field *union*, not a slot list), fixed in the same pass.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed and fixed in one pass. **Why:** found by a full-release code review; a mechanical extraction verified as a no-op against the existing test suites and a real-corpus run. | **substantive** |
