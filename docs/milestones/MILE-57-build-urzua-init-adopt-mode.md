# 57 — Build urzua init adopt mode

> Status: Done
> Stable-Id: 01M1YNN9K5B2V3D7MSGD9HWNG1
> Phase: 0
> Track: roadmap-tracking
> Implements: SPEC-5
> Blocked-on: —

## What

`urzua init` adopt mode: proposes `.urzua/config.toml` from an existing corpus's real directories and header conventions, never moving or rewriting a single record. `--dry-run` byte-identical to the real run.

## Why

The tool has to be adoptable without a rewrite for "one engine instead of a fork per repo" to mean anything -- adopt mode is the only path that doesn't ask an existing corpus to change shape first.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
