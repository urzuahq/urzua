---
Status: Planned
Stable-Id: 01M1Y5JRWC6KDYEC0PBZ3ZPB4C
Phase: '4'
Track: scale-and-analytics
Implements: —
Blocked-on: —
---
# 24 — Governance-health analytics: drift trends, multi-repo graph, org rollup, large-corpus performance

## What

Event-collection-based drift-trend analytics, a cross-repo decision graph, org-level rollup, and incremental performance for large corpora.

## Why

The long-term differentiator: nothing in this project yet measures how governance health changes over time or across repositories; zero RFC coverage yet.

## Done means

- Drift trends are computed from collected events and reported across runs.
- A decision graph spans more than one repository.
- An org rollup summarizes it.
- `check` over a corpus of the declared large size finishes within a stated time budget, with incremental runs faster than full ones.
- Each has a fixture or benchmark.
- The analytics are exercised on urzua's own repository history.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Removed an appeal to research a reader cannot open; the argument now rests on this project's own record that no analytics exist yet. `Status` unchanged. | **structural** |
