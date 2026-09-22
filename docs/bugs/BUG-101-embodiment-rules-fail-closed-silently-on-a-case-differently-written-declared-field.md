---
Stable-Id: 01M33QPKPE2N3KVVE69P6GDRDB
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "a_declared_field_written_under_a_different_case_is_reported_observed_failing, rules.rs"
---
# 101 — embodiment rules fail closed silently on a case differently written declared field

## What was wrong

`declared_slots` (`rules.rs:2062-2077`) decides a record is eligible for the embodiment rules
(`embodiment_locator_exists`, `embodiment_consistency`, `embodiment_locator_promotion_candidate`)
purely from config: does the type's `required_fields`/`known_fields` contain the literal string
`"Realized-by"` or `"Embodiment"`. It never opens the record's header. Each rule body then read the
field with `record.header.get(...)`, treating a miss as `Outcome::Absent` regardless of cause.

A record written `realized-by:` (frontmatter keys are preserved exactly as written) incremented
`population.eligible` but was never examined — `Outcome::Absent` — and **no `Finding` was pushed**.
This was indistinguishable in the report from a record that legitimately never adopted the field: the
only trace was `eligible > examined` in the JSON, which nothing enforced anyone reads or acts on. A
real `Embodiment`/`Realized-by` inconsistency on that record went unchecked, silently, on a rule whose
whole purpose is to check exactly that.

A second, smaller instance of the same conflation: a record whose header did not parse *at all* also
produced `Outcome::Absent` here, when the four-state population model's own vocabulary
(`Outcome::Unreadable`) exists to distinguish "could not be read" from "not adopted".

## Why nothing caught it

The four-state `Outcome` model was built so a rule handed nothing could be told apart from a rule that
ran cleanly (`ADR-55`). It works for the case it was built for — a type that never declared the field.
It did not yet distinguish that from *"the field was declared, and this specific record's header just
didn't have the exact-cased key"*, a different fact about the corpus that `ADR-55`'s own disclosure
principle says should be visible.

Same root cause as `BUG-100`, opposite failure direction: that rule failed open (fabricated a value
that never matched, over-reporting); this one failed closed (dropped the candidate silently,
under-reporting).

## What changed

`ADR-58` decided the shared contract `RFC-40` proposed. A new helper, `declared_value`, reads a
declared field through `Header::read_declared` and maps `Missing` to `Outcome::Absent` and
`Unreadable` to `Outcome::Unreadable` — the two are no longer conflated. All three embodiment rules
now go through it. The silent drop itself is closed by a new rule, `header.field-case-mismatch`
(`ADR-58`), which is the one place a declared field written under a different case is reported, so the
embodiment rules' own `Absent` outcome no longer has to (and no longer can, correctly) carry that
diagnosis.

## References

- `ADR-55` — the disclosure principle this bug violated for one specific miss case.
- `ADR-58` — decides `RFC-40` and is the fix implemented here.
- `BUG-100` — the same root cause, opposite failure direction, fixed in the same pass.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed, with a reproduction and a diagnosis of the shared root cause with `BUG-100`. **Why:** found by a full-release code review. | **substantive** |
> | 2026-09-22 | `Status: Open` → `Fixed`, by `ADR-58`'s decision on `RFC-40`. **Why:** the shared contract closed both this bug and `BUG-100` in one pass. | **substantive** |
