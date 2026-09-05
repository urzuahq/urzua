# 0009 — A policy is enforced at the width it declares; scope's own shape stays open

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-04
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0011 (Accepted)

## Context

RFC-0011 proposed several related ideas: a boundary (a set of paths that publish beyond the corpus)
is a declared fact, not an ad-hoc rule; severity is part of that declaration; "violation" and
"evidence" are separate questions needing separate matchers over the same text; declared width must
be mechanically comparable to matched width, in both directions; and an absence assertion is
unverified until its failure has been observed. §6 additionally proposed that a record's scope is
the same kind of declaration — but left open whether the value should be a path glob or a subject,
explicitly noting it "needs the second corpus" to resolve.

This ADR decides §§1–5 and the fail-open/fail-closed rule, which don't depend on that open question.
§6 is deliberately **not** decided here.

## Decision

In the context of policies (boundaries, prohibitions, absence checks) whose declared coverage can
silently diverge from what their detector actually matches, we decided to accept RFC-0011's core
mechanism: **a declared width must be mechanically compared to the matched width, in both
directions** (declared-wider-than-enforced and enforced-but-undeclared are both defects); **a
detector fails open, a prohibition fails closed** on missing input; and **an absence check ships
with a planted-violation test**, since a check that has never been observed to fail is unverified,
not passing. `urzua-core`'s own purity proof (ADR-0005) and header-closure parser (ADR-0008) already
follow this pattern; this ADR makes it the project's general rule rather than something reinvented
per-feature.

**§6 (is scope a path glob or a subject?) is explicitly carried forward as open**, per RFC-0011's
own recorded uncertainty. Deciding it now would be assuming an answer the evidence doesn't support —
the RFC itself says this needs a second corpus with real rename history to test the glob form
against, which doesn't exist yet. When scope is eventually built, whichever form the data model
uses should be the one that survives that test, not the one decided first.

## Reversibility

The width-comparison and fail-open/fail-closed rules are general engineering discipline, not a
specific schema commitment — cheap to apply broadly and hard to regret. Leaving §6 open costs
nothing now and avoids a schema commitment (scope's field shape) that would be expensive to reverse
once records in the wild declare it one way.

## Consequences

- Every future rule with an absence-check shape (a prohibition, a "no dangling references" check)
  must ship a planted-violation test — this is now a project-wide bar, not a per-rule judgment call.
- A record's `scope` field does not exist yet in the schema. It is blocked on §6, not on this ADR —
  building it before §6 resolves would be assuming the very thing left open.
- The fail-open/fail-closed distinction applies immediately to `urzua-io`'s discovery: a `git`
  subprocess failure must be treated as "could not enumerate," never as "the corpus is empty."

## References

- RFC-0011 — the proposal this partially decides; §6 remains open per the RFC's own text.
- ADR-0005, ADR-0008 — prior instances of the same width-comparison discipline this ADR generalizes.
