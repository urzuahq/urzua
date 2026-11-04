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

Named in roadmap research as the long-term differentiator and the emptiest part of the competitive landscape; zero RFC coverage yet.

## Done means

Drift trends are computed from collected events and reported across runs, a decision graph spans more than one repository, an org rollup summarizes it, and `check` over a corpus of the declared large size finishes within a stated time budget with incremental runs faster than full ones; each has a fixture or benchmark, and the analytics are exercised on urzua's own repository history.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |