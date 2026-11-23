---
Stable-Id: 01M7XZ7VYN1NKQ551VKCJVWC4Y
Status: Planned
Phase: '1'
Track: harness
Blocked-on:
---
# 355 — The hook fallback without a usable urzua is bounded in time and names the release it needs

## What

Decide and build how the Claude Code hook fallback behaves when `urzua` on PATH cannot answer.
Two options are open. One bounds the edit fallback's `.urzua/config.yaml` read with a time limit,
or drops it for a version-only answer. The other has the fallback (or `urzua harness status`) say
which release the installed hooks need when an older binary exits 2, or when an older copy earlier
on PATH shadows a newer one.

## Why

BUG-214 took the configuration read off the session-start and post-command fallbacks, which never
used its verdict. The edit fallback still runs the awk validator on every edit. On one macOS machine
with a 0.4.2 binary, that validator took about 4.5 minutes per run on a 257-line configuration. On
Linux the same run takes under 0.1s, so the cost depends on the platform's awk. The binary was old
because a stale copy earlier on PATH shadowed the installed one; this repository's devbox had the
same shadowing on 2026-11-21. A time limit, a version-only refusal and a PATH diagnostic each change
what an adopter sees, so the choice is a design call, not a mechanical fix.

## Done means

- The chosen behaviour is recorded in an ADR, or in an amendment to the ADR that governs the fallback, citing BUG-214.
- The edit fallback finishes within a stated bound on a configuration this repository's own `.urzua/config.yaml` size, under a platform awk known to be slow (or the bound holds without awk), shown by a test.
- A binary too old for `urzua harness hook` is named as such in the fallback's message, with the release the installed hooks need.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
