---
Status: Planned
Stable-Id: 01M1Y5JVK0Z71FE3H8BG58VM39
Phase: '2'
Track: escalation
Implements: —
Depends-on: MILE-47
Blocked-on: —
---
# 29 — A read-only MCP server

## What

Expose rejected/superseded decisions to agents over MCP.

## Why

No RFC or ADR mentions MCP at all yet; flagged in roadmap research as needing a decision.

## Done means

- An MCP server exposes rejected and superseded decisions read-only to agents, serving a record's status, reasons and supersession chain from the corpus.
- The server cannot write.
- A test lists a rejected decision over the server, and a write attempt is refused.
- An ADR decides the server's shape first, since no record mentions MCP yet.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-47` added. **Why:** `MILE-47` states the MCP position must be decided before the read-only server is built, and `MILE-29`'s Done means requires an ADR to decide the server's shape first. Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
