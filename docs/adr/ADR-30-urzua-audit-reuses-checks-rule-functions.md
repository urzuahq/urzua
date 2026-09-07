# 30 — `urzua audit` reuses `check`'s own rule functions, not a second implementation

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/src/main.rs
> Date: 2026-09-06
> Author: @beauwilliams
> Deciders: @beauwilliams
> Supersedes / Superseded-by: —
> Derives-from: SPEC-1

## Context

SPEC-1 already fully specifies `audit`'s scope: supersession reciprocity (if A supersedes B, B
must point back) and dangling cross-references, reported only, never written — cross-record
reconciliation, kept distinct from `check`'s per-record validation since ADR-18/19 moved
Embodiment out of it. Nothing about *what* `audit` checks is open. What was undecided is *how* it
computes it: `rules::pointer_resolution` and `rules::supersession_reciprocity` already exist as
standalone pure functions and already run as part of `check`'s rule set today — so a naive
implementation risks a second, independently-drifting copy of the same two rules under a different
name.

## Decision

In the context of two rule computations that already exist, facing the choice between reusing them
or re-deriving the same logic under `audit`, we decided **`audit` calls the same
`rules::pointer_resolution` and `rules::supersession_reciprocity` functions `check` calls, restricted
to just those two**, and assembles the result into the same `CheckReport` shape `check` already
emits (ADR-23's one-report-shape contract applies here too — there is no reason for `audit` to
invent a second shape for the same kind of data). Waivers apply identically (ADR-11): a waived
finding stays listed, excluded only from the blocking exit code. `audit` never writes, matching
SPEC-1's own stated rationale — a bulk cross-reference rewrite is a real data-loss risk without a
review step this command doesn't have.

## Reversibility

Trivial to revisit: `audit` is ~20 lines of glue over functions that already exist and are already
tested (`rules.rs`'s own planted-violation tests cover both). Nothing new to maintain.

## Consequences

- A future third caller of either rule (there is already a second: `check`) costs nothing — the
  function is the single source of truth, called from as many command entry points as need it.
- `audit`'s report can show `rules_executed` with only two entries where `check` shows eight,
  correctly reflecting narrower scope rather than a fabricated "ran everything" claim.
- If `audit` and `check` ever need genuinely different reciprocity semantics (e.g. `audit` scoping
  to a subset of paths the way `check` does), that's a real divergence to design then — not
  something this ADR forecloses, since the functions take `&[Record]` and don't care where their
  caller is.

## References

- SPEC-1 — `audit`'s full scope, already specified before this ADR.
- ADR-18/19 — the decision that moved Embodiment out of `audit` and into `check`/`fix`, which is
  why `audit` only has these two rules left.
- `rust/crates/urzua-core/src/rules.rs` — `pointer_resolution`, `supersession_reciprocity`.
