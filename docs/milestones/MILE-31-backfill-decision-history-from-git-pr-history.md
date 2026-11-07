---
Status: Planned
Stable-Id: 01M1Y5JWMX52SZ76Y8B1XCTSQM
Phase: '3'
Track: backfill
Implements: —
Depends-on: MILE-52
Blocked-on: —
---
# 31 — Backfill decision history from git/PR history

## What

Reconstruct decision history from commits/PRs with confidence scoring and a draft-until-confirmed status.

## Why

Named in research as the single biggest adoption unlock, and explicitly out of scope for RFC-1 as written.

## Done means

- A backfill command reconstructs decision records from git and pull request history.
- It scores each with a confidence.
- It writes them as drafts that stay in a draft status until a human confirms.
- A second run is a no-op.
- A fixture history yields the expected drafts with their confidence and none with a decided status.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-52` added. **Why:** `MILE-52` states the concurrency design for the backfill engine is settled before the engine's execution model is built. Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
