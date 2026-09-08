---
Status: Done
Stable-Id: 01M1YNND3FMY2F2A0N3K0FVT3H
Phase: '0'
Track: governance-process
Implements: ADR-24
Blocked-on: —
---
# 63 — Build urzua explain and urzua graph

## What

`urzua explain <path>`: every record whose `Realized-by` names that file as evidence. `urzua graph`: the full `Implements`/`Derives-from`/`Supersedes` relationship graph as data, flagging edges that don't resolve.

## Why

The record graph existed in the schema from the start but nothing surfaced it as queryable data -- `explain` answers "which decisions govern this file," the highest-frequency read an agent or reviewer actually has.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
