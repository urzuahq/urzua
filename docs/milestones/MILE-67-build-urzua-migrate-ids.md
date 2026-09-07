# 67 — Build urzua migrate ids

> Status: Done
> Stable-Id: 01M1YNNF8F2GD7BAQVX9E5Q7K5
> Phase: 0
> Track: roadmap-tracking
> Implements: ADR-3, ADR-21

## What

`urzua migrate ids`: backfills a collision-free `Stable-Id` (ULID) into every record lacking one, without touching the filename or any cross-reference -- the display number stays exactly what it already is. Dry-run by default.

## Why

ADR-3's stable/display split only matters if every record actually has a stable ID -- this is what makes the split real for a corpus that predates it, without forcing a disruptive renumbering to get there.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
