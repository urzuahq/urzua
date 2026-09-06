# 0025 — The forge app: shape decided, build explicitly not next

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0014 (Accepted)

## Context

RFC-0014 proposes a forge app as the destination for governance-event collection (RFC-0003 §5
already established a repo cannot maintain its own history under normal branch protection) and the
mechanism several roadmap features (escalation, cross-repo graph, attestation identity) need. The
RFC itself names "building it now" as a non-goal — the CLI's own record validation has to be real
and exercised first, which it now is, but that doesn't obligate building the collector next; it
only removes the reason this RFC couldn't be decided yet.

## Decision

In the context of a proposal whose own text separates "decide the shape" from "build it now," facing
several roadmap features that are forge-shaped rather than tree-shaped, we decided to accept the
shape now while explicitly not committing to building it: **no write access to repository
contents** (read-only contents/metadata, write only on the app's own surfaces — check runs, PR
comments, review requests); **the CLI stays fully correct with no app installed**, offline, no
account; **it is one adapter behind a forge-agnostic interface** (observe events, publish a finding,
request a human, enumerate repositories), not the architecture; **it stores events and derived
summaries, never customers' actual decision-record text**.

## Reversibility

The shape is cheap to have decided now and expensive to retrofit later once history/escalation/
cross-repo features are built against an ad hoc mechanism instead. Nothing about this decision
creates code, a dependency, or a running service — it constrains what gets built whenever it is.

## Consequences

- Any future escalation, cross-repo-graph, or attestation-identity feature that needs a forge
  presence builds against this adapter shape, not a bespoke mechanism per feature.
- No repository-contents write permission is ever requested by this component, structurally —
  ADR-level, not just documented intent.
- Real open questions stay open, not resolved by this ADR: whether the app is self-hostable or
  centrally hosted, event retention/deletion policy, how it authenticates a *decider* rather than
  just proving the app acted, and whether its existence should be allowed to influence the still-
  unanswered escalation question (Q1) — it explicitly should not, per RFC-0014 §5's own caution
  against building escalation just because a mechanism now exists.
- Sequencing is unchanged: this remains not-next. Nothing in the current roadmap depends on it yet.

## References

- RFC-0014 — the proposal this decides.
- RFC-0003 §5 — the collection-needs-a-destination argument this shape answers.
- RFC-0002 §2 — the identity tiering an authenticated forge account would improve on.
