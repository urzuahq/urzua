---
Stable-Id: 01M6JW47AQ6W7550ZN46VXHF2B
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 167 — The revision-log section identity is declared in configuration, not hardcoded

## What

Replace the literal `**Revision log**` in Rust source with a section identity declared in configuration, defaulting to the current literal.

## Why

`ADR-64` makes config-driven the whole thesis, yet the revision-log section is found by one hardcoded string. A record whose prose contains that literal makes the parser read the wrong table, and a repository that names the section differently cannot use the log at all.

## Done means

The identity is declared per type in `.urzua/config.yaml`; a record whose prose mentions the literal elsewhere is not misread; the default keeps existing repositories unchanged; a fixture covers a renamed section and a prose mention and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
