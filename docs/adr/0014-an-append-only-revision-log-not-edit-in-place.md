# 0014 — An append-only revision log, not edit-in-place

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0006 (Accepted)

## Context

RFC-0006 proposed that a Spec (and an ADR) is never silently overwritten: every substantive change
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
explicitly deferred to its own follow-up RFC, per RFC-0006 itself.

## Consequences

- Every record type gains a revision-log section with `change_class` as a required field on each
  entry — a missing tag is a blocking finding, not a warning, since an unclassified change is
  exactly the failure mode (substantive changes folded silently into "structural") this ADR exists
  to prevent.
- RFC-0008's eligibility test (clause 4: reversible) depends on this revision log existing — every
  tool-authored write RFC-0008 permits appends a `structural` revision entry here.
- The substantive/structural boundary is not fully specified — RFC-0006 flags it as needing worked
  examples before the rule is trustworthy. Until then, judgment calls on ambiguous cases are a human
  decision, not a mechanical one.
- The ADR-vs-revision-log boundary (when does drift require a new ADR rather than a revision entry)
  is explicitly out of scope here and deferred to its own follow-up RFC.

## References

- RFC-0006 — the proposal this decides.
- RFC-0005 — Embodiment's `Drift detected` state, a candidate trigger for "drifted enough to need a
  new ADR," not resolved here.
- RFC-0008 — depends on this revision log for its own reversibility requirement.
