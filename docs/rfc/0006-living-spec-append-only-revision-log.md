# 0006 — The living-spec problem: an append-only revision log, not edit-in-place

> Status: Draft
> Date: 2026-07-30
> Author: (session author)
> Supersedes / Superseded-by: —

> **Split from RFC-0001, 2026-07-30.** This was originally §4 of
> `0001-unified-schema-and-layering.md`. Split so it's independently acceptable/rejectable from
> the core schema shape, the layering model, and the Embodiment/claim-graph model. Content is
> unchanged from the original §4, per this repo's own rule against silent rewrites.

## Summary

This is an open design question that a naive edit-in-place model never fully solves. Propose an
append-only revision log for Specs (and ADRs): a record is never silently overwritten, every
substantive change is a new revision entry, the current rendered view is always the latest
revision, and the full history stays queryable — so "what did we originally think, and when did
reality diverge, and why" is always answerable.

## Motivation

"What happens to a Spec once you're implementing it and discover the plan was wrong" is easy to
leave unanswered until it causes damage. A spec retrofit review can find silent Status-value
rewrites and vanished sections precisely because there's no defined mechanism for a Spec to change
*honestly* mid-implementation. This RFC has to answer that, not just flag it.

**The tension:** a decision record's core value proposition is that it's a trustworthy historical
record — "why did we build it this way" needs to still be answerable in two years. But a Spec is
also supposed to be the living, current truth of what's being built — and reality diverges from
plan constantly during implementation (that's not a failure state, it's normal). A concrete case of
this tension going wrong: `Status` values silently rewritten during what was framed as a purely
structural retrofit, with no record of *why* the value changed — a spec reading `Ready` three lines
below its own banner saying the described threshold gate is not what actually got built.

## Proposal

**Proposed model: append-only revision log, not edit-in-place.**

- A Spec (and an ADR, for that matter) is never silently overwritten. Every substantive change is
  a new revision entry: `{date, author, summary_of_change, diff_or_pointer}`.
- The *current* rendered view of a Spec is always the latest revision — so it stays genuinely
  "living" for someone reading it today.
- But the full revision history stays queryable — so "what did we originally think, and when did
  reality diverge, and why" is always answerable, which is the actual thing an immutable audit
  trail needs to guarantee. **Immutability lives at the revision-log level, not at the
  current-document level** — you don't need to freeze the *document*, you need to never lose a
  *prior version of it*.
- A revision that changes something *substantive* (a `Status` value, a claimed guarantee, a
  security boundary) vs. something *cosmetic* (heading rename, whitespace) should be
  distinguishable in the log — directly addressing the retrofit failure mode above (substantive
  changes folded silently into a "structural, not content" characterization). Propose a required
  `change_class: substantive | structural` tag on every revision entry, enforced the same way any
  other required field is (blocking if missing).
- **When a Spec drifts far enough from its governing ADR that the ADR's decision itself no longer
  holds**, that's not a Spec update — it's a new ADR (superseding or amending the old one). The
  revision log handles "the plan details changed, the decision still holds"; a new ADR handles
  "the decision itself needs to change." Getting this boundary right is probably the single
  hardest part of this whole model and deserves its own follow-up RFC once there's a real
  implementation to test it against, rather than resolving it definitively here.

**Reconstructability claim this enables:** "the whole system should basically be producible from
just the ADRs, RFCs, and Specs" becomes literally true under this model — the current state is the
latest revision of every record, and the full history
(including every discarded RFC, every superseded ADR, every Spec revision) is the append-only log
underneath it. Nothing needs a separate "changelog" document; the record IS the changelog, always.

## Open questions

- Where exactly is the substantive/structural boundary for revision-log tagging? Evidence from real
  retrofit reviews shows this is genuinely hard to call correctly even by a careful human reviewer
  — this may need worked examples before the rule is trustworthy, not just a two-value enum.
- What does "Spec drifted enough to need a new ADR" actually look like operationally — is this a
  judgment call surfaced to a human, or can Embodiment's existing `Drift detected` alarm state
  (RFC-0005) double as the trigger?
- The ADR-vs-revision-log boundary noted above ("the single hardest part of this whole model")
  remains genuinely unresolved and is explicitly deferred to its own follow-up RFC.

## Non-goals

Does not attempt to design the paging/notification mechanism, dashboard/UI, or storage technology
— same scope boundary as RFC-0001. Does not attempt to resolve the ADR-vs-revision-log boundary
question above — flagged as needing its own follow-up RFC, not solved here.

## References

- RFC-0001 — the core schema/profile model this revision-log mechanism attaches to.
- RFC-0005 — Embodiment's `Drift detected` state, referenced above as a candidate trigger for "Spec
  drifted enough to need a new ADR."
