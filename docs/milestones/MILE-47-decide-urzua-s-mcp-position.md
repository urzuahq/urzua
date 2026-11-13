---
Status: Done
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

- A decision record states urzua's MCP position: whether to ship an MCP server, why read-only if so, and which data it exposes (rejected and superseded decisions, through a generic status filter). The maintainer chose an ADR with no separate RFC on 2026-11-13. — evidence: code:docs/adr/ADR-99-urzua-ships-a-read-only-mcp-server-as-a-stdio-transport-over-its-existing-reports.md@sha256:cebd6a67fb88f6139999702c92ab04a657a0823d7c0bf2ddbaa931adadaa5c92
- The record is filed under `docs/adr/` and passes `urzua check`. — evidence: code:docs/adr/ADR-99-urzua-ships-a-read-only-mcp-server-as-a-stdio-transport-over-its-existing-reports.md@sha256:cebd6a67fb88f6139999702c92ab04a657a0823d7c0bf2ddbaa931adadaa5c92, code:.github/workflows/checks.yml
- `MILE-29` cites it in `Implements`, so the build milestone no longer rests on an unrecorded stance. — evidence: code:docs/milestones/MILE-29-a-read-only-mcp-server.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Kind: decision` added. **Why:** maintainer decision of 2026-11-07 (`ADR-96` amendment, milestone kinds): this milestone records a decision, so it is judged against a decided record (an Accepted decision or a Shipped spec) rather than a built tier. Its `Status` is computed from its Done means items by `urzua fix`, not set by hand. | **structural** |
> | 2026-11-13 | Blank line removed after the revision-log header row, so the tool can append rows. **Why:** `urzua reverify` and `fix` refused to append to a log whose table was split by a blank line. | **structural** |
> | 2026-11-13 | Done means reworded from an RFC to a decision record. **Why:** the maintainer decided on 2026-11-13 that `ADR-99` decides the MCP position without a separate RFC, taking the recommendation on each choice; `MILE-29` cites it in `Implements`. | **structural** |
> | 2026-11-13 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (2 of 3 items resolved, 1 waived) computes to Implemented` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-13 | The "MILE-29 cites it" item now cites `MILE-29`'s file as evidence instead of a waiver, and the "passes `urzua check`" item adds the CI workflow that runs it. **Why:** ADR-96 reserves a waiver for a result no file can evidence, and `MILE-29`'s `Implements: ADR-99` is in a file. | **structural** |
