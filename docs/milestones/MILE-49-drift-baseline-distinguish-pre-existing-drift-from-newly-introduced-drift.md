---
Status: Planned
Stable-Id: 01M1YN86A7Z8N7YP5F89Q5Q04K
Phase: '1'
Track: embodiment-model
Implements: RFC-5
Blocked-on: —
---
# 49 — Drift baseline: distinguish pre-existing drift from newly-introduced drift

## What

Distinguish pre-existing drift (present before Urzua was ever run against a corpus) from newly-introduced drift (introduced since the last clean run), so adopting Urzua on an existing corpus doesn't block on a backlog of drift nobody caused today.

## Why

`embodiment.consistency` currently reports every drifted record identically regardless of when the drift happened. A corpus adopting Urzua for the first time (MILE-51) could show substantial pre-existing drift that would make `check` unusable as a merge gate on day one unless the two are distinguishable.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
