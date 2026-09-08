# 61 — Build urzua fix detect and apply modes

> Status: Done
> Stable-Id: 01M1YNNC28J6KNP3D6PC6H7SCY
> Phase: 0
> Track: embodiment-model
> Implements: ADR-15, ADR-19, ADR-20
> Blocked-on: —

## What

`urzua fix`: detect mode reports fields whose stated value disagrees with what's computed from `Realized-by` evidence; `fix --apply --ids <records> --by <you>` writes the computed value back, one field's line only, with a required identity and an appended revision-log entry. Refuses outright on a record with no revision log.

## Why

`check` can tell you a field is wrong; nothing closed the loop by writing the mechanically-correct value back. Apply mode is gated hard (explicit ids or --force, a resolved identity, an auditable revision-log entry) because a bulk unreviewed write is exactly the failure mode a governance tool can't afford.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
