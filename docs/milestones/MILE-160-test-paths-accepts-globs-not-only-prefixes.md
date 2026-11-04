---
Stable-Id: 01M6JW43RMC4HFW0TD4PBBVFZS
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 160 — `test_paths` accepts globs, not only prefixes

## What

Let the `test_paths` setting (`MILE-146`) take glob patterns such as `**/*.test.*` as well as directory prefixes.

## Why

Test files in a real adopter corpus sit beside the code they test and are named by suffix, not collected under one directory, so a prefix list cannot name them. `MILE-158` gave the diff path options globs for the same reason. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A `test_paths` entry that contains a glob metacharacter is matched as a glob and any other entry stays a prefix; an entry that matches no file is reported rather than silently ignored; a fixture covers a prefix and a glob together and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
