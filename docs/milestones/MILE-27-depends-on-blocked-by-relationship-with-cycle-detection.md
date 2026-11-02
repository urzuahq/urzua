---
Status: Planned
Stable-Id: 01M1Y5JTH1RN024BFKVMV0Q6XF
Phase: '1'
Track: review-workflow
Implements: —
Blocked-on: RFC-48
---
# 27 — Depends-on/Blocked-by relationship with cycle detection

## What

A genuine sequencing relationship beyond Phase/Track tags, checked for resolution and cycles.

## Why

Named explicitly in ADR-34/SPEC-6 as deferred until a real case needs it -- Phase/Track tags are sufficient for the backlog as it exists today.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | `Blocked-on` moved from the trigger to `RFC-48`. **Why:** The trigger fired: planning Phase 0 needed an order `Phase`/`Track` could not express; `RFC-48` proposes the design. | **substantive** |
