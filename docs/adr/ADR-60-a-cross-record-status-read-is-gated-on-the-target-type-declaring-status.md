---
Stable-Id: 01M3461YQSAZGTR3GP66ZXQQB4
Status: Accepted
Date: 2026-09-22
Author: beauwilliams
Deciders: beauwilliams
Derives-from: BUG-109
---
# 60 — a cross-record Status read is gated on the target type declaring Status

## Context

`claim_status_agreement`, `pointer_target_status`, and `narrative_field_stale` all read a claim,
pointer, or narrative reference's *target* record's
`Status` field via a hardcoded literal, unconditional on whether the target's own type declares
`Status` in `required_fields`/`known_fields`. `header.field-case-mismatch` (`ADR-58`), by contrast,
only scans a type's actually-declared fields, per `ADR-53`'s "declared, not voted" principle.

The gap (`BUG-109`): a record of a type that never declares `Status` writing `status:` instead of
`Status:` is correctly invisible to `header.field-case-mismatch` (the field isn't declared vocabulary
for that type) but all three still try to read `Status` on it
regardless, silently treat the miss as unjudged, and skip. The case-mismatch is real and nothing
reports it.

This is pre-existing behavior, not something `RFC-40`/`ADR-58` introduced — both rules hardcoded a
`"Status"` lookup with no declaration check before this release. `RFC-40` decided what a miss *means*
once a lookup happens; it never decided *whether* the lookup should happen at all for a type that
never declared the field.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Gate the read: skip a target record if its type does not declare `Status`** | Consistent with `ADR-53`; no new special case for one field; a type that never adopted `Status` is treated the same as a type that never adopted any other field | A type relying on the current unconditional check (declaring no `Status` but writing one anyway, matched today by accident) loses coverage until it declares the field |
| **Reserve `Status` like `Stable-Id`/`Rule`/`Scope`/`Expires`, fold its case via `get_reserved`** | No adopter action required; the field "just works" regardless of declaration | Contradicts `ADR-57` everywhere else `Status` is read (`header.required-fields`, `header.field-set-consistency` both treat it as exact-match adopter vocabulary); makes `Status` uniquely privileged among adopter-facing fields for no principled reason — it is not an engine-invented name the way `Stable-Id` is |
| **Leave it** | Nothing to build | The gap stands, and a case-mismatched `Status` field goes both unenforced (all three rules) and undiagnosed (the field simply isn't declared, so `header.field-case-mismatch` has nothing to scan either) |

## Decision

In the context of three rules reading a hardcoded `"Status"` regardless of the target's declared
vocabulary, facing a choice between exempting `Status` from `ADR-53`'s declaration principle or
holding it to the same rule as every other field, **we decided to gate the read on declaration**: a
record whose type does not declare `Status` in `required_fields`/`known_fields` is treated by all
three exactly as a declaration miss — unjudged, not judged-and-wrong — to keep `Status` an ordinary
piece of adopter vocabulary rather than a second reserved-field exception `ADR-57`'s own reasoning
does not support, accepting that a type which has never declared `Status` loses all three rules'
checks on its records entirely.

A type that wants `pointer.target-status`/`claim.status-agreement`/`narrative-field.stale` to judge a referenced record's status must
declare `Status` in `required_fields` or `known_fields` — the same requirement every other
declaration-gated rule already imposes, and the one `header.field-case-mismatch` already assumes.

## Reversibility

Cheap to reverse: the gate is a single condition at each of the three call sites. Reverting returns to
the pre-`ADR-60` unconditional read, which is `BUG-109`'s own defect, so reverting would need new
evidence that gating causes a worse problem than the one it closes.

## Consequences

A type that never declares `Status` in `required_fields`/`known_fields` no longer has its records'
`Status` field checked by any of the three rules at all — a real narrowing,
symmetric with `header.field-case-mismatch`'s own scope. An adopter who wants these checks declares
`Status` like any other field they want governed. `identity.collision` and other rules untouched by
this decision are unaffected.

## References

- `BUG-109` — the defect this ADR decides.
- `ADR-53` — "declared, not voted"; the principle this decision applies uniformly to `Status`.
- `ADR-57`, `ADR-58` — the exact-match and miss-contract decisions this ADR extends rather than
  reopens.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Decided. **Why:** `BUG-109` named two options and needed a real decision before a fix; gating on declaration was chosen because the alternative (reserving `Status`) would have made it a uniquely privileged field with no principled reason `ADR-57` supports elsewhere. | **substantive** |
