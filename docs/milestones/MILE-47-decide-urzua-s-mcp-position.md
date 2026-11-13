---
Status: Planned
Stable-Id: 01M1YN858DBHWE5XJ2JFEVY1F6
Phase: '2'
Track: escalation
Kind: decision
Blocked-on: —
---
# 47 — Decide Urzua's MCP position

## What

Write an RFC deciding Urzua's MCP position: whether to ship an MCP server at all, and if so, why read-only (exposing rejected/superseded decisions so an agent doesn't re-propose what was already tried) rather than write-capable.

## Why

MILE-29 already names "a read-only MCP server" as a build target, but no RFC or ADR backs that stance -- unlike every other build-track milestone, its own `Implements` field is empty. The position (why read-only, why this specific data) needs deciding before the build, not assumed by the milestone's own title.

## Done means

- An RFC states urzua's MCP position: whether to ship an MCP server, why read-only if so, and which data it exposes (rejected and superseded decisions).
- The RFC is filed under `docs/rfc/` and passes `urzua check`.
- `MILE-29` cites it in `Implements`, so the build milestone no longer rests on an unrecorded stance.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Kind: decision` added. **Why:** maintainer decision of 2026-11-07 (`ADR-96` amendment, milestone kinds): this milestone records a decision, so it is judged against a decided record (an Accepted decision or a Shipped spec) rather than a built tier. Its `Status` is computed from its Done means items by `urzua fix`, not set by hand. | **structural** |
