---
Stable-Id: 01M6DEN8ZYA5Z4DVW6E7SA10F3
Status: Planned
Phase: '0'
Track: schema-governance
---
# 132 — Declare a type's status values and the fields each status requires

## What

Let a type declare its allowed Status values and, per status, the fields a record in that status must carry (for example, an Accepted ADR needs `Decision-date`; a Fixed bug needs a fix reference). A rule reports an undeclared status value and a missing status-dependent field.

## Why

Status vocabularies are repeated today as lists inside individual rule options (`terminal_statuses`, `closed_statuses`) in `.urzua/config.yaml`, so they drift apart and nothing rejects a misspelled status. `urzua new` also writes `Status: null` because the type has no declared initial status. One declaration on the type fixes both and gives the rules a single source.

## References

- `.urzua/config.yaml` -- `terminal_statuses` and `closed_statuses` duplicated across rule options.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
