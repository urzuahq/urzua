---
Status: Planned
Stable-Id: 01M1Y5JTH1RN024BFKVMV0Q6XF
Phase: '1'
Track: review-workflow
Implements: —
---
# 27 — Depends-on/Blocked-by relationship with cycle detection

## What

A genuine sequencing relationship beyond Phase/Track tags, checked for resolution and cycles.

## Why

Named explicitly in ADR-34/SPEC-6 as deferred until a real case needs it -- Phase/Track tags are sufficient for the backlog as it exists today.

## Done means

A `Depends-on`/`Blocked-by` relationship is declared in config as a pointer field, `check` reports an unresolved target and a cycle (a planted cycle fixture), and the relationship orders this repository's milestones beyond `Phase` and `Track`; `RFC-48` is Accepted and its design is what is built, and the capability is exercised in urzua's own `.urzua/config.yaml` and milestones.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Blocked-on: RFC-48` removed. **Why:** `RFC-48` is accepted; this milestone is buildable. | **substantive** |
> | 2026-11-02 | `Blocked-on` moved from the trigger to `RFC-48`. **Why:** The trigger fired: planning Phase 0 needed an order `Phase`/`Track` could not express; `RFC-48` proposes the design. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
