# 69 — Stdout is always JSON; remove the --format flag

> Status: Done
> Stable-Id: 01M1YNNG9RQEZYDDHBQDN5AJDW
> Phase: 0
> Track: governance-process
> Implements: ADR-23

## What

Removed `--format`: `check`, `fix`, `explain`, `graph`, and `migrate schema --report` now emit exactly one JSON object on stdout, unconditionally, with no human-readable rendering anywhere.

## Why

Two output shapes mean two things that can silently drift apart, and an agent piping stdout, a script, and a person reading a terminal should all see exactly the same thing -- one shape is a stronger guarantee than a flag that happens to default correctly.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
