---
Status: Planned
Stable-Id: 01M1YN88CCNH4KVBE7PKGY30CJ
Phase: '4'
Track: scale-and-analytics
Kind: decision
Blocked-on: —
---
# 53 — Decide whether a hosted service can reconstruct corpus history retroactively

## What

Decide whether a hosted component could reconstruct a corpus's governance history retroactively (replaying `check` over historical commits) rather than only collecting forward from install, and if so, resolve the open questions that gate it: replay cost, which tool version to replay with, determinism (RFC-5's own staleness model has time-dependent terms), and squashed-history coarsening.

## Why

MILE-24 names "drift trends" generically; this is the specific, harder question underneath it -- whether a trend can start with real history on day one instead of a flat line, and what a hosted service would have to get right to make that trustworthy rather than a demo. A non-goal today (RFC-14 already declines a hosted corpus-holding service), but the concrete case any future ADR on that topic would have to weigh.

## Done means

- An ADR decides whether a hosted service may reconstruct corpus history by replaying `check` over historical commits, or records the decision to decline with the reason.
- The ADR answers the replay-cost question in the What.
- The ADR answers the question of which tool version replays.
- The ADR answers the determinism question given `RFC-5`'s time-dependent staleness terms.
- The ADR answers the squashed-history coarsening question.
- `RFC-14`'s non-goal on a hosted corpus-holding service is cited.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Kind: decision` added. **Why:** maintainer decision of 2026-11-07 (`ADR-96` amendment, milestone kinds): this milestone records a decision, so it is judged against a decided record (an Accepted decision or a Shipped spec) rather than a built tier. Its `Status` is computed from its Done means items by `urzua fix`, not set by hand. | **structural** |
