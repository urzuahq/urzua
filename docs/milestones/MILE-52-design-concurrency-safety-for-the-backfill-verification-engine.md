---
Status: Planned
Stable-Id: 01M1YN87VGAYN7092E3W0P4S9D
Phase: '3'
Track: backfill
Blocked-on: —
---
# 52 — Design concurrency safety for the backfill/verification engine

## What

Design how the eventual backfill/verification engine (MILE-31) runs concurrent units of work against a repository without racing -- real, confirmed worktree isolation per unit versus a queueing/locking model -- before that engine is built.

## Why

Backfill at real scale is concurrent agents working the same repository by design, not an edge case. A race in that mechanism, discoverable only by luck rather than a guaranteed property of the system, isn't acceptable in a tool whose entire value is being trustworthy. Needs its own design pass once the engine's execution model is being built, not assumed safe by default.

## Done means

A design note or ADR states how concurrent units of the backfill and verification engine (`MILE-31`) run against one repository without racing, choosing worktree isolation per unit or a queue and lock model with the reasoning, and a test or prototype with concurrent writers shows no lost update and no torn read before the engine's execution model is built on it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |