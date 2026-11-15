---
Stable-Id: 01M7FW3RWEY15CZPYB2EJ4MG3N
Status: Planned
Phase: '1'
Track: roadmap-tracking
Blocked-on: null
---
# 262 — The roadmap reports a critical path per phase

## What

Make `urzua roadmap` report the critical path of each phase, not one path for the whole corpus.

## Why

the audit of 2026-11-15 found the roadmap reporting a path through later-phase milestones while the phase in progress has its own unfinished bars.

## Done means

- `urzua roadmap` returns a critical path for each phase with open milestones.
- For this repository it names the open exit-bar milestones of the phase in progress.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
