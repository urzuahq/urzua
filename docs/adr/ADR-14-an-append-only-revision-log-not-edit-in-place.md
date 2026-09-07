# 14 — An append-only revision log, not edit-in-place

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-6 (Accepted)

## Context

RFC-6 proposed that a Spec (and an ADR) is never silently overwritten: every substantive change
is a new revision entry, the current rendered view is always the latest revision, and the full
history stays queryable. The alternative — edit-in-place — cannot answer "what did we originally
think, and when did reality diverge, and why," which is the actual property a decision-record system
exists to guarantee.

## Decision

In the context of records that are supposed to be both a trustworthy historical artifact and the
living, current truth of what's being built, facing a naive edit-in-place model's inability to be
both at once, we decided that **immutability lives at the revision-log level, not at the
current-document level**: every revision entry carries `{date, author, summary_of_change,
diff_or_pointer}` and a required `change_class: substantive | structural` tag, enforced the same way
any other required field is — blocking if missing.

A revision that changes something substantive (a `Status` value, a claimed guarantee, a security
boundary) must be distinguishable in the log from something cosmetic (heading rename, whitespace).
The current document is never frozen; the sequence of prior versions is.

## Reversibility

Additive to the existing schema: a revision log is a new construct records carry, not a change to
how existing fields are read. The genuinely hard question this ADR does not resolve — where a Spec's
drift from its governing ADR crosses into needing a new ADR rather than a revision entry — is
explicitly deferred to its own follow-up RFC, per RFC-6 itself.

## Consequences

- Every record type gains a revision-log section with `change_class` as a required field on each
  entry — a missing tag is a blocking finding, not a warning, since an unclassified change is
  exactly the failure mode (substantive changes folded silently into "structural") this ADR exists
  to prevent.
- RFC-8's eligibility test (clause 4: reversible) depends on this revision log existing — every
  tool-authored write RFC-8 permits appends a `structural` revision entry here.
- The substantive/structural boundary is not fully specified — RFC-6 flags it as needing worked
  examples before the rule is trustworthy. Until then, judgment calls on ambiguous cases are a human
  decision, not a mechanical one.
- The ADR-vs-revision-log boundary (when does drift require a new ADR rather than a revision entry)
  is explicitly out of scope here and deferred to its own follow-up RFC.

## Amendment (2026-09-07): permanent spec numbers, a real Why, and completeness at every revision

While extending SPEC-6 with a terminal milestone status, a new spec number (SPEC-7) was minted for
what was actually a change to SPEC-6 itself, on the reasoning that editing an already-`Accepted`,
already-executed spec in place would repeat RFC-6's own admitted ambiguity. That was a
misapplication of this ADR, not a gap in it: this ADR already answers the question it was trying to
route around — immutability lives at the revision-log level, not the current-document level, so an
executed spec is not frozen, it's revised. The question this ADR actually defers is different (when
drift crosses into needing a new *ADR*), and stays deferred below. SPEC-7 is retracted; its content
folds into a SPEC-6 version bump under the rule this amendment states.

Separately, this corpus's actual revision-log entries have so far been terse one-liners stating what
changed without why — weaker accountability than an ADR's full-prose amendment, and a real gap
against this ADR's own claim that immutability "moved to the log" rather than being lost.

Decided:

1. **A spec's own number is permanent per subject.** A substantive or structural change to an
   already-`Accepted` spec is a `Version` bump plus a revision-log entry on that same spec, never a
   new spec number. A new number is reserved only for a genuinely distinct subject — "the existing
   spec already shipped" is never by itself a reason to mint a new one.
2. **A `substantive`-classified revision-log entry must state a real Why**, not just what changed —
   the reason, gap, or decision that drove the change — in addition to what. A `structural` entry can
   stay a one-line what.
3. **A spec's body must always read as a complete, self-sufficient specification of its subject as
   it exists today, not an incremental delta on the original text.** When a spec is revised, the
   revision-log entry (what, why, dated) and the spec's own body update together, so someone working
   from the spec alone, replaying it end-to-end in a different codebase, would arrive at the same
   thing without ever reading the revision history. The log carries the accountability; the body
   carries the current, replayable truth.

This deliberately keeps Spec practice (edited in place, accountability in the log) distinct from ADR
practice (visible dated Amendment sections, decision text never edited): an ADR is a point-in-time
decision fact, where silently changing the text would erase the record of why code looks the way it
does; a spec is the living current build-truth for its subject, where the same current-truth-only
view is exactly what an implementer needs. No new field beyond the already-required `change_class`.
Does not require rewriting past terse entries — applies going forward.

## References

- RFC-6 — the proposal this decides.
- RFC-5 — Embodiment's `Drift detected` state, a candidate trigger for "drifted enough to need a
  new ADR," not resolved here.
- RFC-8 — depends on this revision log for its own reversibility requirement.
- MILE-73 — "decide when a spec needs a new spec instead of an in-place revision," resolved by the
  amendment above.
