# 68 — Build urzua migrate schema --report

> Status: Done
> Stable-Id: 01M1YNNFS3073SZAAP376CD603
> Phase: 0
> Track: schema-governance
> Implements: ADR-22
> Blocked-on: —

## What

`urzua migrate schema --report --field <Name>`: a read-only preview listing every record that would newly fail if `Name` were added to `required_fields` today, before it's ever added to config.

## Why

Adding a required field to a live corpus is a real risk of silently breaking every record that doesn't have it yet -- this answers "what would break" before the config change that would cause it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
