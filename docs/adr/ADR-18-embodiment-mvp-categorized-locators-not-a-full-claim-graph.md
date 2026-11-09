---
Stable-Id: 01M6R3NK47Y21MYE4XT7H9WBQ0
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules/embodiment.rs#RealizedBy@sha256:f42331a3d48ccc70210d1765e21d28448f18c0632d9631e5859f6ae4c5cda55f, code:rust/crates/urzua-core/src/rules/embodiment.rs#parse_realized_by@sha256:e21e21a9b60dfca7670642514142f7e77a5dd0aad0819860648c4414f96ba9f5, code:rust/crates/urzua-core/src/rules/embodiment.rs#compute_embodiment@sha256:337ee455b133de479b08a6a8aa5a77596e10120d41610cbf5764e2b5462bb0b3, code:rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_consistency@sha256:4e306b1bc61e982967d0a0c065e7b369111433854c51c61ce0ef3ed69f7bdca1, code:rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_locator_promotion_candidate@sha256:460dc50417cd886199c0cd0fd3a74f94f73b36e8d777674e68877e1eeeb488b5, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_test_locator_outranks_a_code_locator@sha256:a47ebe9b171d5093f2b3c476b576b011df75aa50275169b1130777370600c1a5, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_locator_cited_by_two_records_is_a_promotion_candidate_observed_failing@sha256:9a114e09e65381e0c2161d88a0bcb8544e76c91803fc0ae5213ba5e5fa890dee, test:rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_disagreeing_with_realized_by_is_a_planted_violation_observed_failing@sha256:6a33dfc2241251821033cb04dd3e8e8ee22602a43b7cde35f0bab362db23e057
Date: 2026-09-05
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-5
Non-reference-ids: ADR-72
---
# 18 — Embodiment MVP: categorized locator lists, not a full claim graph

## Context

RFC-5 proposed lifting Embodiment into the core schema with `realized_by` as a claim graph:
AND/OR composites, weakest-link confidence, cross-record-shared promoted nodes, cycle detection.
RFC-5 itself already stages this — "a first linter implementation only needs to handle the
no-sharing, single-record tree case." No real precedent for the full graph model was found; a
working realization-tracking implementation checked against uses a far simpler shape: three
categorized locator lists with a fixed tier priority, no composites at all. Building the full graph
now would be speculative complexity with nothing yet to justify it, exactly what this project's own
practice argues against.

## Decision

In the context of RFC-5's Embodiment model needing a first real implementation, facing a choice
between building the full claim graph immediately or the simplest model that actually computes
something useful, we decided the MVP is: `Realized-by` holds categorized locators (`spec:`, `code:`,
`test:` prefixes), and Embodiment is computed as the highest tier with at least one non-empty
category — any `test:` locator means `Verified`; else any `code:` locator means `Implemented`; else
any `spec:` locator means `Specified`; else `Not started`. A stated `Embodiment` that disagrees with
the computed value is a finding.

**Promotion is real but narrow.** When the same locator is cited by more than one record's
`Realized-by`, `check` surfaces it as a finding rather than silently letting three records drift
independently on the same underlying evidence (the ADR-72/73/74 shape RFC-5 names). A human
runs the actual promotion into a first-class `claim` record (its own type, its own directory,
checked and audited the same as any other record — the same precedent ADR-11 already established
for waivers). Promotion is never automatic.

**Explicitly not in this MVP**: AND/OR composite operators, weakest-link (minimum) confidence,
cycle detection, a `NOT` operator, and per-locator staleness verification against git history (RFC-
0005's tier 4). Each ships only once a real case is found that needs it — RFC-5 remains the
record of what the fuller model looks like when that happens.

## Reversibility

Additive and narrow by construction: `Realized-by` is a new header field, `claim` a new record type,
neither replaces anything existing. The categorized-list shape is a strict subset of RFC-5's full
graph — a bare leaf is "the overwhelmingly common shape" RFC-5 itself already calls out — so
growing into composites later is additive to the schema, not a breaking change to this MVP's format.

## Consequences

- A new rule, `embodiment.consistency`: parses `Embodiment` and `Realized-by`, computes the expected
  tier, and reports disagreement. Non-blocking (`Warning`) to start, since this is the first run of a
  brand-new check against a corpus that has never been held to it — consistent with how the two Phase
  A seed rules were introduced.
- A new rule, `embodiment.locator-promotion-candidate`: flags a locator cited by more than one
  record's `Realized-by` as a promotion candidate. Reports only; never rewrites `Realized-by` itself
  (RFC-8's eligibility test would need to classify this as tool-writable first, and it doesn't
  obviously qualify — promotion changes what a record's field *means*, not just its stale value).
- `Realized-by` parsing and the tier computation are pure functions in `urzua-core` — no filesystem or
  git access needed for this MVP, since staleness-against-history (tier 4) is explicitly deferred.
  This keeps the purity boundary (ADR-5/6) untouched by this decision.
- A `claim` record type needs a directory, a config entry, and its own required-field set before
  promotion has anywhere to land — tracked as follow-up work, not designed here.

## References

- RFC-5 — the proposal this scopes into a first shippable slice.
- ADR-11 — the `waiver`-as-first-class-record precedent this reuses for `claim`.
- RFC-8/ADR-15 — the eligibility test that explains why promotion isn't tool-authored.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 809e4004d. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs` a49eedaabcd0 -> 6b61cdfe6465. Reason: Re-verified: compute_embodiment still takes the highest non-empty tier over the same five states; the changes since the pin (per-field severity, grace window, sticky states, test back-pointers as a test tier) are additive. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-04 | Added the `Non-reference-ids` header field declaring `ADR-72`. **Why:** the text writes "ADR-72/73/74" as a worked example from another corpus, and a real `ADR-72` here would otherwise capture that citation (`BUG-190`, `ADR-75`). The field carries no decision; the Decision is unchanged. | **structural** |
> | 2026-11-05 | Narrowed to the item the decision concerns; re-pin approved by the maintainer's 2026-11-04 drift decision (`ADR-74`, `MILE-185`): `embodiment.rs` to `RealizedBy` (lines 4-13), `compute_embodiment` (lines 82-106), `embodiment_consistency` (lines 270-442), `embodiment_locator_promotion_candidate` (lines 488-581), and added `parse_realized_by` (lines 46-67), where the `spec:`/`code:`/`test:` categories are read; `embodiment.rs` to `a_test_locator_outranks_a_code_locator` (lines 1099-1106), `a_locator_cited_by_two_records_is_a_promotion_candidate_observed_failing` (lines 1174-1191), `embodiment_disagreeing_with_realized_by_is_a_planted_violation_observed_failing` (lines 1027-1046). **Why:** an unrelated edit to a volatile file must not stale this pin again; the spans were chosen by the author of this change and not reviewed line by line by the maintainer. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs#parse_realized_by` (lines 46-63) 3219fb88b918 -> e21e21a9b60d. Reason: Re-verified against the Decision: parse_realized_by now takes its entries from locator::realized_by_entries (BUG-129, maintainer decision 2026-11-05: a comma separates Realized-by entries only before a category prefix). Every Realized-by value that parsed before parses the same; only a comma inside a path changes. The pull request reviewer is the human decision. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-06 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_locator_promotion_candidate` (lines 561-688) 6b7a34f0cea9 -> 0b1fb0ddb5f6, `rust/crates/urzua-core/src/rules/embodiment.rs#a_locator_cited_by_two_records_is_a_promotion_candidate_observed_failing` (lines 1359-1376) b76b718bb4a5 -> 9a114e09e653. Reason: embodiment_locator_promotion_candidate gains a scope: Option<&ChangedOnly> parameter and, when set, reports a shared locator only on a changed citer (ADR-90); its test passes None for the new parameter. Reporting of a locator cited by two records is unchanged without the option, so the evidence still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_locator_promotion_candidate` (lines 561-692) 0b1fb0ddb5f6 -> 944fe16b5359. Reason: embodiment_locator_promotion_candidate now emits one finding per changed citer under scope: changed (ADR-90 amendment point 5) instead of one on the first; the citer census and the rule's contract are unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_consistency` (lines 448-623) 46203f5609d8 -> abf19b57ccf8, `rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_locator_promotion_candidate` (lines 669-800) 944fe16b5359 -> 407246ae3e60. Reason: embodiment_consistency and embodiment_locator_promotion_candidate now read the evidence field through evidence_value, which appends the locators of a record's Done-means item clauses (ADR-96, MILE-214) to its Realized-by when the record's type declares statuses.delivery.items and is the Realized-by value unchanged for every other type; the highest-non-empty-tier computation compute_embodiment performs and the per-citer promotion census this decision records are unchanged. The line ranges move because embodiment.rs gained LocatorEnv and locator_defect above them, extracted from embodiment_locator_exists. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_consistency` (lines 551-623) abf19b57ccf8 -> 4e306b1bc61e, `rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_locator_promotion_candidate` (lines 833-904) 407246ae3e60 -> 460dc50417cd. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: embodiment.rs: split embodiment_consistency into the ConsistencyJudge methods judge, audit_date_finding and claim_age_finding plus disagreement_finding, embodiment_locator_promotion_candidate into LocatorCiters (cite, promotion_findings), and locator_defect into locator_kind, locator_kind_defect and locator_scope_defect with the LocatorDefect alias; the checks keep their order and messages. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline and reverify JSON compared before and after, with and without --as-of). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
