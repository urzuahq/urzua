---
Stable-Id: 01M36CH8TTHSJF7T4V4V8B4X4X
Status: Accepted
Date: 2026-09-23
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-45
---
# 63 — a rule population discloses every non-absent exclusion and no rule depends on another to do it

## Context

`RFC-45` proposed closing two related gaps in `Population`'s disclosure: the `ADR-60` declared-not-voted
ambiguity, and `BUG-125`'s live, reproduced defect (a rule's silence about `Unreadable` depending on a
sibling opt-in rule). It left naming and a scope question open.

## Options considered

| Question | Options | Decision driver |
|---|---|---|
| Declared-not-voted disclosure | Reuse `Outcome::OutOfScope` vs. a new disclosed category | A new category is precise and doesn't stretch `OutOfScope`'s existing, narrower meaning (identity/prefix shape) to also cover cross-record field declaration — two concepts that happen to share a sentence in their doc comment but aren't the same failure mode |
| `Unreadable` disclosure | Leave as an undisclosed remainder vs. its own `Population` count | `BUG-125` is live and reproduced; disclosure is the fix itself — an adopter who can see the number decides whether to also enable `header.required-fields`, informed rather than blind |
| Scope of the audit | Individually audit every rule that can produce `Unreadable` vs. fix the shared `census`/`census_records` machinery once | The machinery is shared by every `Field`/`Record`-unit rule already; fixing it there closes the gap for all of them in one change, matching this codebase's own established preference (`census`/`Population` itself exists to stop exactly this kind of per-rule hand-maintenance divergence) |

## Decision

In the context of two related disclosure gaps — one a review false-positive, one a live defect — facing
open questions on naming and scope, **we decided**:

- **A new `Outcome` variant, `Outcome::Undeclared`**, returned by `declared_cross_record_value` when
  the target's type doesn't declare the field being read. Distinct from `Absent` (field declared,
  simply not written).
- **`Population` gains two new disclosed counts, `unreadable` and `undeclared`**, alongside the
  existing `eligible`/`examined`/`out_of_scope`, computed structurally by `census`/`census_records`
  from the `Outcome` each candidate returns — never hand-maintained per rule, the same discipline that
  already governs the other three.
- **`Absent` stays the implicit remainder** (`eligible - examined - out_of_scope - unreadable -
  undeclared`): the one state that is self-evidently benign on its own terms, needing no further
  disclosure.
- **Fixed once, in the shared machinery**, not audited rule-by-rule: any rule whose body can return
  `Outcome::Unreadable` or `Outcome::Undeclared` is automatically covered the moment `census`/
  `census_records` tally the new counts, closing `BUG-125`'s shape for every current and future rule
  without a per-rule migration.
- **The general principle, stated as policy governing all future rule design**: no rule's correctness
  or honesty may depend on another rule being enabled. `ADR-53` guarantees no particular subset of
  `ALL_RULES` is ever enabled together; a rule's own `Population` must be self-sufficient under any
  subset, including the empty one alongside it.

## Amendment (2026-09-23, same day)

The first bullet above — a new `Outcome::Undeclared` variant threaded into `Population` — did not
survive implementation and is superseded by this note rather than silently rewritten (`AGENTS.md`).
Building it surfaced that the ambiguity in `pointer_target_status`/`narrative_field_stale`/
`claim_status_agreement` lives one level below what a single `Outcome` per `census` candidate can
express: the candidate `census` tracks is the *source* record's own declared slot, genuinely
`Examined` once read; whether the *target* it resolves to can be judged is a nested, secondary
question a new top-level `Outcome` variant cannot reach. A `Finding`-based fix was tried next and
rejected on review as "every caller invents its own check" one level down from what
`header.field-case-mismatch` already prevents for a declared-field case mismatch.

**Corrected decision**: Part 1 ships as one new, dedicated, opt-in rule,
`relation.target-status-undeclared` — for every resolved pointer/narrative reference, does the
target's type declare `Status`. The three original rules are unchanged. `Outcome::Undeclared` was
removed; `Population` gains only `unreadable` (Part 2, unaffected by this correction).
`claim_status_agreement`'s claim-sourced references are out of the new rule's scope (`RFC-45`'s
Non-goals) — a distinct data shape, not evidenced as a live gap.

## Reversibility

Two independent, separately reversible changes. `Population.unreadable` is additive to the report
schema — no existing consumer's currently-read fields change meaning. Reverting it means removing the
one count and returning to the prior collapse, which reopens `BUG-125` and the original review
ambiguity; cheap structurally, expensive in the defect it reopens. `relation.target-status-undeclared`
is a new, opt-in rule: reverting it means deleting the rule and its config entry, with no effect on
`Population` or on the three original rules it does not modify.

## Consequences

`Population::detailed`'s constructor signature grows by one argument (`unreadable`); every call site
is updated in the same change, verified by the compiler. `SPEC-2`'s documented report shape gains one
field. A new rule, `relation.target-status-undeclared`, joins `ALL_RULES`. This repository's own
corpus has no unreadable headers and no undeclared-target references today, so the new count and the
new rule both read/report clean — a real-corpus `check` diff before/after confirms this rather than
assuming it. The `AGENTS.md` "A narrower rule population is usually policy, not a regression" section
is removed once this ships — a structural fix supersedes a prose stopgap rather than sitting beside it
(`ADR-54`'s reasoning, applied to an internal process note instead of an adopter-facing format).

## References

- `RFC-45` — the proposal this ADR decides.
- `BUG-125` — the live defect this ADR's `unreadable` disclosure closes.
- `ADR-53`, `ADR-55`, `ADR-60` — the principles this ADR makes structural rather than assumed.
- `RFC-40`/`ADR-58` — `header.field-case-mismatch`'s "one dedicated rule" precedent, the shape this
  ADR's corrected Part 1 decision follows.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Decided. **Why:** `RFC-45` named two related gaps and needed both naming and scope decided before implementation; resolved toward fixing the shared `census`/`census_records` machinery once rather than auditing every rule individually, and stated the "no rule depends on another rule" principle explicitly as the governing rule for all future rule design, not just this instance. | **substantive** |
> | 2026-09-23 | Amended, same day: Part 1's `Outcome::Undeclared` decision did not survive implementation (see Amendment section) and is corrected to one dedicated rule instead, per `AGENTS.md`'s "don't silently rewrite an Accepted decision." | **substantive** |
