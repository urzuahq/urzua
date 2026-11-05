---
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/audit.rs@sha256:e0d748e6add22082edad3a816355b73db7b79ae7254fecda7dfd3ae0df1ec664, test:rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_0_on_reciprocated_supersession@sha256:655cea3e0dcf4219d4467524a19e66f97ed82cf0ac5473f723b8170edd5d53cc, test:rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing@sha256:c7b4d65b1cbe805c72ee5d4641a9f7022a97dc4292693bf411257375b5e08591, test:rust/crates/urzua-cli/tests/check_integration.rs#audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally@sha256:3d463bcd88a7060201bd6018312a234e8c9027465f152e7baf869e1a5a8ad473
Date: 2026-09-06
Accepted-on: 2026-09-06
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: SPEC-1
---
# 30 — `urzua audit` reuses `check`'s own rule functions, not a second implementation

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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 5c573cf70. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/commands/audit.rs` 9f471810cf72 -> e0d748e6add2, `rust/crates/urzua-cli/tests/check_integration.rs` 288a08b371a8 -> b668d62f2f4a. Reason: Re-verified: audit.rs diff is the single fail_on line; it still calls the same two rule functions as check. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Narrowed to the item the decision concerns; re-pin approved by the maintainer's 2026-11-04 drift decision (`ADR-74`, `MILE-185`): `check_integration.rs` to `audit_exits_0_on_reciprocated_supersession` (lines 361-391), `audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing` (lines 393-431), `audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally` (lines 433-489). **Why:** an unrelated edit to a volatile file must not stale this pin again; the spans were chosen by the author of this change and not reviewed line by line by the maintainer. | **structural** |
