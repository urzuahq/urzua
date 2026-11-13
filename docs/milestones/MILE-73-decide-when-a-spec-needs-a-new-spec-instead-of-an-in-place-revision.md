---
Status: InProgress
Stable-Id: 01M1YSGQ9F9WXDWB2VJEQ657K4
Phase: '1'
Track: governance-process
Kind: decision
Implements: RFC-6, ADR-14
Blocked-on: —
Realized-by: spec:docs/adr/ADR-14-an-append-only-revision-log-not-edit-in-place.md, spec:docs/specs/SPEC-1-cli.md
Non-reference-ids: SPEC-7
---
# 73 — Decide when a spec needs a new spec instead of an in-place revision

## What

Resolve RFC-6's own admitted open question: when a spec has already shipped (`Status: Accepted`,
already executed) and new capability needs documenting, is the right move an in-place revision
(the living-spec model RFC-6 currently describes) or a new spec (leaving the executed one frozen)?
RFC-6's own Open Questions section names this exactly ("what does 'spec drifted enough to need a
new ADR' actually look like operationally") and explicitly defers it to a follow-up RFC rather than
answering it. This milestone answers the new-spec-or-revise question only; the drift-to-ADR question RFC-6 also names stays open and `MILE-50` holds it.

## Why

Hit directly, live: SPEC-7 was written as a new spec rather than a revision to SPEC-6 specifically
because RFC-6 doesn't say which is correct once a spec has executed. That call was made once, for
one case, without a real decision behind it -- the next time this comes up, nothing says whether to
follow the same precedent or do something else. Worth deciding properly rather than accumulating
inconsistent precedent one spec at a time.

## Done means

- `ADR-14` is amended to say when a spec gets a new number and when it is revised in place: a spec's number is permanent per subject, a substantive or structural change is a `Version` bump plus a revision-log entry on the same spec, and a new number is reserved for a genuinely distinct subject. — evidence: spec:docs/adr/ADR-14-an-append-only-revision-log-not-edit-in-place.md
- `RFC-6`'s open question of when a spec has drifted enough to need a new ADR stays open: the amended `ADR-14` leaves it deferred, and `MILE-50` is the record that tracks and will decide it. — evidence: spec:docs/adr/ADR-14-an-append-only-revision-log-not-edit-in-place.md, spec:docs/milestones/MILE-50-decide-whether-drift-detected-doubles-as-the-new-adr-needed-trigger.md
- The spec minted for this case was retracted. — evidence: spec:docs/adr/ADR-14-an-append-only-revision-log-not-edit-in-place.md, spec:docs/specs/SPEC-1-cli.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Resolved via an amendment to ADR-14 (not a new RFC): a spec's own number is permanent per subject, a substantive/structural change is a `Version` bump plus a revision-log entry on the same spec, and a new number is reserved only for a genuinely distinct subject. SPEC-7 (minted for this exact case, live) is retracted and folded into SPEC-6 v0.2 under this rule. **Why:** RFC-6 named this as an open question and deferred it; the live SPEC-7 case forced an actual answer rather than letting it accumulate as inconsistent precedent one spec at a time. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-05 | Added the `Non-reference-ids` header field declaring `SPEC-7`. **Why:** the text names this id as history or illustration, not as citations of a record; the declaration states that on the record itself, so a new unresolved citation in this file is reported, where a file-wide `citation.resolution` waiver had silenced every one (`BUG-190`, `ADR-75`). The field carries no decision; the milestone is unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 2. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
> | 2026-11-07 | Left open: the item `RFC-6`'s open question points at the amended `ADR-14` rule is not made true. **Why:** `RFC-6`'s open questions ask when a spec has drifted enough to need a new ADR, and `ADR-14`'s amendment of 2026-09-07 says in terms that this is the question it leaves deferred; the amendment answers a different one (a new spec number or an in-place revision), which `RFC-6` never lists as open. Pointing the open question at the amendment would say it is resolved when it is not, so no pointer was added. Whether the item should name the question `RFC-6` actually asks, or be dropped as resting on a misreading, is the maintainer's call. | **substantive** |
> | 2026-11-07 | The item "`RFC-6`'s open question points at the amended `ADR-14` rule" is amended to what is true. Maintainer decision of 2026-11-07. **Why:** the original claim was wrong. `RFC-6` asks when a spec has drifted enough to need a new ADR, and `ADR-14`'s amendment answers a different question (a new spec number or an in-place revision) and says it leaves that one deferred, so `RFC-6`'s question was never resolved by it and cannot point at it. The question stays open and `MILE-50`, which implements `RFC-6`, tracks it; the item now states that and cites `ADR-14` and `MILE-50`. | **substantive** |
> | 2026-11-07 | `Kind: decision` added. **Why:** maintainer decision of 2026-11-07 (`ADR-96` amendment, milestone kinds): this milestone records a decision, so it is judged against a decided record (an Accepted decision or a Shipped spec) rather than a built tier. Its `Status` is computed from its Done means items by `urzua fix`, not set by hand. | **structural** |
> | 2026-11-08 | The What now says the drift-to-ADR question RFC-6 names is not decided here and `MILE-50` holds it. **Why:** the What read as if this milestone resolved all of RFC-6's open question, while its Done means (amended 2026-11-07) deliberately leaves the drift question to `MILE-50`; the text now agrees with the item. | **structural** |
