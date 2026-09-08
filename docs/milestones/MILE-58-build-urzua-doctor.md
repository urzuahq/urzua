---
Status: Done
Stable-Id: 01M1YNNAAMTDJMEHET3W5XK1QE
Phase: '0'
Track: roadmap-tracking
Blocked-on: —
---
# 58 — Build urzua doctor

## What

`urzua doctor`: reports on the tool's own configuration and invocation health (does a config exist, does it parse, is `check` actually wired into CI) rather than on record content.

## Why

A checker that's correct but never invoked is indistinguishable from no checker at all -- doctor answers "is this actually running," a different question than "did this find anything."

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
