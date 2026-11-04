---
Status: Done
Stable-Id: 01M1YSGQ9F9WXDWB2VJEQ657K4
Phase: '1'
Track: governance-process
Implements: RFC-6, ADR-14
Blocked-on: —
Realized-by: spec:docs/adr/ADR-14-an-append-only-revision-log-not-edit-in-place.md
---
# 73 — Decide when a spec needs a new spec instead of an in-place revision

## What

Resolve RFC-6's own admitted open question: when a spec has already shipped (`Status: Accepted`,
already executed) and new capability needs documenting, is the right move an in-place revision
(the living-spec model RFC-6 currently describes) or a new spec (leaving the executed one frozen)?
RFC-6's own Open Questions section names this exactly ("what does 'spec drifted enough to need a
new ADR' actually look like operationally") and explicitly defers it to a follow-up RFC rather than
answering it.

## Why

Hit directly, live: SPEC-7 was written as a new spec rather than a revision to SPEC-6 specifically
because RFC-6 doesn't say which is correct once a spec has executed. That call was made once, for
one case, without a real decision behind it -- the next time this comes up, nothing says whether to
follow the same precedent or do something else. Worth deciding properly rather than accumulating
inconsistent precedent one spec at a time.

## Done means

`ADR-14` is amended to say when a spec gets a new number and when it is revised in place: a spec's number is permanent per subject, a substantive or structural change is a `Version` bump plus a revision-log entry on the same spec, and a new number is reserved for a genuinely distinct subject; `RFC-6`'s open question points at it, and the spec minted for this case was retracted.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Resolved via an amendment to ADR-14 (not a new RFC): a spec's own number is permanent per subject, a substantive/structural change is a `Version` bump plus a revision-log entry on the same spec, and a new number is reserved only for a genuinely distinct subject. SPEC-7 (minted for this exact case, live) is retracted and folded into SPEC-6 v0.2 under this rule. **Why:** RFC-6 named this as an open question and deferred it; the live SPEC-7 case forced an actual answer rather than letting it accumulate as inconsistent precedent one spec at a time. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
