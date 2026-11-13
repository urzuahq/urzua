---
Status: Planned
Stable-Id: 01M1YN85S6E9J8SEM91Y7QDNS5
Phase: '1'
Track: governance-process
Kind: decision
Blocked-on: —
---
# 48 — Decide whether Urzua adopts an executable-rule tier

## What

Write an RFC deciding whether Urzua interoperates with an executable-rule tier (a decision paired with a mechanically-enforced, blocking check) as a complement to Embodiment, or explicitly declines to build one.

## Why

Embodiment (RFC-5) verifies after the fact; an executable rule prevents at commit, for the subset of decisions expressible as a code check. Undecided whether Urzua positions Embodiment as the fallback for decisions that can't be executable rules, competes on executable rules directly, or cedes that ground entirely -- currently no record answers this at all.

## Done means

- An RFC decides whether urzua interoperates with an executable-rule tier as a complement to Embodiment or declines to build one.
- The RFC names which of the three positions (fallback for what cannot be executable, direct competition, or ceding the ground) it takes, with the reasoning.
- The RFC is filed under `docs/rfc/` and passes `urzua check`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Kind: decision` added. **Why:** maintainer decision of 2026-11-07 (`ADR-96` amendment, milestone kinds): this milestone records a decision, so it is judged against a decided record (an Accepted decision or a Shipped spec) rather than a built tier. Its `Status` is computed from its Done means items by `urzua fix`, not set by hand. | **structural** |
