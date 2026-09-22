---
Stable-Id: 01M33YAYC6AZ4P9K5GEA910BBQ
Status: Fixed
Found-in: "A /code-review 104 pass over round-15-field-case-mismatch"
Regression-test: "a_claim_against_a_target_whose_type_never_declares_status_is_not_judged_observed_failing, rules.rs"
---
# 109 — Status is read as a hardcoded universal field by two rules instead of the type-declared vocabulary ADR-53 requires

## What was wrong

`claim_status_agreement`, `pointer_target_status` and `narrative_field_stale` all read a target
record's `Status` field via a hardcoded literal, unconditional on whether the target's *type* actually
declares `Status` in `required_fields`/`known_fields`. `header.field-case-mismatch` (`ADR-58`), by
contrast, only scans fields a type actually declares, per `ADR-53`'s "declared, not voted" principle.

The gap: if a record type doesn't declare `Status`, a record of that type writing `status:` instead
of `Status:` is invisible to `header.field-case-mismatch` (correctly — the field isn't declared
vocabulary for that type) but the three cross-record rules still tried to read `Status` on it
regardless, silently treated the miss as unjudged, and skipped. The case-mismatch was real and
undetected by anything.

## Why nothing caught it

This was pre-existing behavior, not something `RFC-40`/`ADR-58` introduced: all three rules hardcoded
a `"Status"` lookup with no declaration check before this release, and nothing in `ADR-57`/`RFC-40`
touched *whether* the lookup should be gated by declaration — only *what a miss means* once the lookup
happens. A `/code-review` pass on the `RFC-40`/`RFC-39` branch surfaced it as a residual gap in the
miss contract.

## What changed

`ADR-60` decided the question this bug left open: a cross-record `Status` read is gated on the
target's own type declaring `Status`, the same as any other field. A new helper,
`declared_cross_record_value`, wraps `declared_value` with that check and is used by all three rules.
A type that never declares `Status` loses these three rules' checks on its records entirely — the same
narrowing `header.field-case-mismatch` already imposes.

## References

- `ADR-53` — "declared, not voted"; the principle this bug's gap was measured against.
- `ADR-57`, `ADR-58` — the exact-match and miss-contract decisions this bug is adjacent to.
- `ADR-60` — the decision that closes this bug.
- `header.field-case-mismatch` — the rule whose declaration-scoped design made the inconsistency
  visible.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed, not fixed. **Why:** found by code review on the RFC-40/RFC-39 branch; the fix required a design decision (gate the lookup, or formally reserve `Status`) that neither RFC decided, so filing it rather than picking one under review pressure. | **substantive** |
> | 2026-09-22 | Fixed by `ADR-60`. **Why:** the user asked to fix this before the 0.4.0 release; `ADR-60` decided to gate the read on declaration rather than reserve `Status`, since reserving it would contradict `ADR-57` everywhere else `Status` is read. | **substantive** |
