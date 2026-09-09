---
Version: '0.3'
Date: 2026-09-07
Status: Accepted
Author: '@beauwilliams'
Subject: '`urzua audit` -- cross-record reconciliation: supersession reciprocity and dangling references.'
Implements: ADR-30
Parent: SPEC-1
---
# SPEC-11 — `urzua audit`

## Purpose

Cross-record reconciliation, kept distinct from `check`'s per-record validation: supersession
reciprocity (if A supersedes B, B must point back) and dangling cross-references. Read-only, always
— a bulk cross-reference rewrite is a real data-loss risk without a review step this command doesn't
have (the same incident SPEC-2's original `--fix`-deferral cites).

## What it runs

`audit` calls the exact same `rules::pointer_resolution` and `rules::supersession_reciprocity`
functions `check` calls — restricted to just those two — rather than re-deriving the same logic
under a different name. `relation.supersession-reciprocity` is not `audit`-exclusive: `check` runs
it too (see SPEC-2), and `audit`'s value is narrowing to only cross-record concerns, not owning
supersession reciprocity outright.

Supersession reciprocity is **status-aware**: a claim only binds once the claiming record is itself
in a terminal-accepted state — a `Draft` ADR claiming to supersede an `Accepted` one doesn't yet
require the older record to point back.

## Output

Assembled into the same `CheckReport` shape `check` already emits (ADR-23's one-report-shape
contract) — no second shape invented for the same kind of data. `rules_executed` correctly shows two
entries where `check` shows ten, reflecting narrower scope rather than a fabricated "ran everything"
claim. Waivers apply identically to `check` (ADR-11/SPEC-10): a waived finding stays listed, only
excluded from the blocking exit code.

## Reuse, not reimplementation

A future third caller of either rule function costs nothing — the function is the single source of
truth, callable from as many command entry points as need it. If `audit` and `check` ever need
genuinely different reciprocity semantics (e.g. `audit` scoping to a subset of paths the way `check`
does), that is a real divergence to design deliberately, not something ruled out by sharing the
function today, since both take `&[Record]` and neither cares where its caller runs from.

## What's deliberately not built

- **Writing.** Not deferred pending infrastructure (unlike `fix`, SPEC-8) — permanently out of
  scope. The reciprocity checker must never write; a bulk rewrite is the exact incident this
  project's own practice cites as the reason repair needed a revision log before it could exist at
  all (`fix`, not `audit`, is where that repair path lives, gated hard).
- **Path scoping** (`audit docs/adr/` narrowing which records are cross-checked) — `check` has this
  (BUG-1's fix); `audit` doesn't yet, since nothing has needed it.

## References

- ADR-30 — the decision this spec details: reuse, not a second implementation.
- SPEC-1 — `audit`'s scope, fully specified before ADR-30 decided the mechanism.
- SPEC-2 — `check`, which shares `relation.supersession-reciprocity` with `audit`.
- ADR-18/19 — the decision that moved Embodiment out of `audit` and into `check`/`fix`, leaving
  `audit` with only these two rules.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, giving `audit` its own complete build document rather than leaving it as an ADR describing only the *how*, with the *what* living separately in SPEC-1. **Why:** MILE-77 found this and `new`/`explain`/`graph`/`migrate` documented only as ADRs while comparable-complexity areas (`check`, `init`) had specs. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
