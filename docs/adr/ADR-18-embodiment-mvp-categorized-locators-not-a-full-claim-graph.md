---
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules.rs, test:rust/crates/urzua-core/src/rules.rs
Date: 2026-09-05
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: RFC-5
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
