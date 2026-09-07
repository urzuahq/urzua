# 52 — Design concurrency safety for the backfill/verification engine

> Status: Planned
> Stable-Id: 01M1YN87VGAYN7092E3W0P4S9D
> Phase: 1
> Track: backfill

## What

Design how the eventual backfill/verification engine (MILE-31) runs concurrent units of work against a repository without racing -- real, confirmed worktree isolation per unit versus a queueing/locking model -- before that engine is built.

## Why

Backfill at real scale is concurrent agents working the same repository by design, not an edge case. A race in that mechanism, discoverable only by luck rather than a guaranteed property of the system, isn't acceptable in a tool whose entire value is being trustworthy. Needs its own design pass once the engine's execution model is being built, not assumed safe by default.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
