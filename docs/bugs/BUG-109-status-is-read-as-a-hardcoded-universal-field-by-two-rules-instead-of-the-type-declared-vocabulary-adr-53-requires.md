---
Stable-Id: 01M33YAYC6AZ4P9K5GEA910BBQ
Status: Open
Found-in: "A /code-review 104 pass over round-15-field-case-mismatch"
Regression-test: "not yet written -- needs a design decision first, see below"
---
# 109 — Status is read as a hardcoded universal field by two rules instead of the type-declared vocabulary ADR-53 requires

## What was wrong

`claim_status_agreement` and `pointer_target_status` both read a target record's `Status` field via
`declared_value(target, "Status")` — a hardcoded literal, unconditional on whether the target's
*type* actually declares `Status` in `required_fields`/`known_fields`. `header.field-case-mismatch`
(`ADR-58`), by contrast, only scans fields a type actually declares, per `ADR-53`'s "declared, not
voted" principle.

The gap: if a record type doesn't declare `Status`, a record of that type writing `status:` instead
of `Status:` is invisible to `header.field-case-mismatch` (correctly — the field isn't declared
vocabulary for that type) but `claim_status_agreement`/`pointer_target_status` still try to read
`Status` on it regardless, silently treat the miss as `Absent`, and skip. The case-mismatch is real and
undetected by anything.

## Why nothing caught it

This is pre-existing behavior, not something `RFC-40`/`ADR-58` introduced: both rules hardcoded a
`"Status"` lookup with no declaration check before this release, and nothing in `ADR-57`/`RFC-40`
touched *whether* the lookup should be gated by declaration — only *what a miss means* once the lookup
happens. A `/code-review` pass on the RFC-40/RFC-39 branch surfaced it as a residual gap in the miss
contract, but the root question (should `Status` be treated as adopter-declared vocabulary like every
other field, or as a semi-reserved convention this engine reads regardless of declaration?) predates
this release and was never decided either way — it was simply never inconsistent enough to notice
before `header.field-case-mismatch` existed to draw the contrast.

## What this needs before a fix

A decision, not a patch: either (a) `claim_status_agreement`/`pointer_target_status` gate their
`Status` read on the target type declaring it, accepting that a type which never declares `Status`
loses these checks entirely on its records, or (b) `Status` is formally declared a reserved,
engine-read field like `Stable-Id`/`Rule`/`Scope`/`Expires` (`ADR-57`'s `get_reserved` precedent),
which would mean case-*folding* it rather than exact-matching it — a real behavior change `ADR-57`
would need to bless explicitly, since `Status` is currently adopter vocabulary everywhere else
(`header.required-fields`, `header.field-set-consistency`). Either path is a design decision, not
implied by anything already decided.

## References

- `ADR-53` — "declared, not voted"; the principle this bug's gap is measured against.
- `ADR-57`, `ADR-58` — the exact-match and miss-contract decisions this bug is adjacent to but does not
  reopen.
- `header.field-case-mismatch` — the rule whose declaration-scoped design makes the inconsistency
  visible.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed, not fixed. **Why:** found by code review on the RFC-40/RFC-39 branch; the fix requires a design decision (gate the lookup, or formally reserve `Status`) that neither RFC decided, so filing it rather than picking one under review pressure. | **substantive** |
