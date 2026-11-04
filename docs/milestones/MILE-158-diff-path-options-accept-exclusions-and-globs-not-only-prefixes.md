---
Stable-Id: 01M6JKNW1YB9CAQ7EMKFQE6HMW
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 158 — Diff path options accept exclusions and globs, not only prefixes

## What

Let the `diff.*` path options take globs and exclusions so a set such as every source file except test files can be stated.

## Why

The options are prefixes only, so when tests sit beside the code in `*.test.*` files there is no way to say everything except tests. The rule `diff` is meant to answer "did code change without a governing record"; a test-only change must not trip it, and the configuration cannot say so. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A path option may declare an exclusion or glob; a diff touching only an excluded file is not a trigger; prefix-only configurations behave unchanged; fixtures cover tests beside code; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
