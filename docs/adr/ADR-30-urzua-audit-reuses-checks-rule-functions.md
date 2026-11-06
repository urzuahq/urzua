---
Stable-Id: 01M6R3NK4JJR22Q8JMDDWWPN9Y
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/audit.rs@sha256:80228f66e936a465168f1f2a4e8cbca8070fb1dfd42a28c087af2eeaec6d7a5e, test:rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_0_on_reciprocated_supersession@sha256:5c0c0a88d6158590703fa1e8bb055eaeb8124db58c6ed5109fc56d9088e220ae, test:rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing@sha256:33724f9c7b6da2a5d560336d17bcdae7a3f558ee2b06e409b6addaf9613b6a6f, test:rust/crates/urzua-cli/tests/check_integration.rs#audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally@sha256:60fd2f4ae8378d956ed2d8a41ae62bfb70201e8a496c2d508bf0c8b66956f217
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

## Amendment (2026-11-05): `audit` no longer has a narrower rule set than `check`

*Status of this amendment: recorded 2026-11-05 as a consequence of `ADR-79` (Accepted); it adds no decision of its own. `Status` stays `Accepted` and the Decision text above is unchanged.*

The Decision restricts `audit` to `rules::pointer_resolution` and `rules::supersession_reciprocity`, and the
Consequences say `audit` shows `rules_executed` with two entries where `check` shows eight. `ADR-79` decided
that `audit` owns no rule set: it is a deprecated alias that runs `check`'s engine over the rules the
configuration declares, so its findings equal `check`'s. The claim that `audit`'s rule set is narrower than
`check`'s is **superseded by `ADR-79`**. What survives is the part this ADR was about: the reciprocity and
reference logic stays one function each, called from `check`, and `audit` reaches it only by delegating to
`check`. A narrower report is `check --rules pointer.resolution,relation.supersession-reciprocity`.

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
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/commands/audit.rs` e0d748e6add2 -> 32666b0d2a62. Reason: BUG-74 renamed Waiver to Suppression (and load_waivers to load_suppressions) in this file's internals; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/commands/audit.rs` 32666b0d2a62 -> 80228f66e936, `rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing` (lines 394-437) c7b4d65b1cbe -> 398946f1eb04. Reason: audit.rs now runs check's engine over the configured rules with a deprecation notice, and Command::Audit's help describes the deprecated alias (ADR-79 as amended, MILE-190); the supersession-reciprocity behaviour ADR-30 cites is unchanged, now reported through check, and the pinned test body changed only to match **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_0_on_reciprocated_supersession` (lines 343-371) 655cea3e0dcf -> 5c0c0a88d615, `rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing` (lines 373-414) 398946f1eb04 -> 33724f9c7b6d, `rust/crates/urzua-cli/tests/check_integration.rs#audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally` (lines 416-471) 3d463bcd88a7 -> 60fd2f4ae837. Reason: The cited tests only changed how their fixture directory is owned: each now holds a common::TempDir guard that removes it on drop instead of calling remove_dir_all (BUG-193); the assertions and the audit behaviour they pin are unchanged. The pin on audit.rs is not touched. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Corrects the row of 2026-11-05 that re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing` (lines 394-437) c7b4d65b1cbe -> 398946f1eb04 (`#311`), whose reason was cut off mid-sentence ("the pinned test body changed only to match") and hid what changed. Accurate reason: audit no longer owns a narrower cross-record rule set; the pinned test now asserts audit's findings equal check's. Reciprocity detection is unchanged, but ADR-30's 'narrower than check' no longer holds (ADR-79). The pinned test dropped its assertion that `header.required-fields` is absent from audit's output, which was the evidence for the narrower rule set. **Why:** an independent check of the re-pin reasons found the earlier row incomplete or inaccurate; corrected here without rewriting it. | **structural** |
> | 2026-11-05 | Corrects the same `#311` row's reason for `rust/crates/urzua-cli/src/commands/audit.rs` 32666b0d2a62 -> 80228f66e936: `audit.rs` now delegates wholesale to `check` (it calls `check::run` with one deprecation `Notice` and owns no rule set; it no longer calls `pointer_resolution` or `supersession_reciprocity` itself). The earlier row's phrase "runs check's engine over the configured rules" is accurate but did not say that nothing of audit's own remains. **Why:** an independent check of the re-pin reasons found the earlier row incomplete or inaccurate; corrected here without rewriting it. | **structural** |
> | 2026-11-05 | Amendment added: the claim that `audit`'s rule set is narrower than `check`'s is superseded by `ADR-79`. **Why:** `ADR-79` (Accepted) decided that `audit` owns no rule set, and the corrected pin reasons above record the same; the amendment keeps the Decision text honest. `Status` stays `Accepted` and the Decision text is unchanged. | **substantive** |
