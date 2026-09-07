# 49 — Drift baseline: distinguish pre-existing drift from newly-introduced drift

> Status: Planned
> Stable-Id: 01M1YN86A7Z8N7YP5F89Q5Q04K
> Phase: 0
> Track: embodiment-model
> Implements: RFC-5

## What

Distinguish pre-existing drift (present before Urzua was ever run against a corpus) from newly-introduced drift (introduced since the last clean run), so adopting Urzua on an existing corpus doesn't block on a backlog of drift nobody caused today.

## Why

`embodiment.consistency` currently reports every drifted record identically regardless of when the drift happened. A corpus adopting Urzua for the first time (MILE-51) could show substantial pre-existing drift that would make `check` unusable as a merge gate on day one unless the two are distinguishable.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
