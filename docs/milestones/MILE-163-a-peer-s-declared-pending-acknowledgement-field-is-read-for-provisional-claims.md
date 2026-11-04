---
Stable-Id: 01M6JW45AMX9BBJY1VQHP51ZH2
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 163 — A peer's declared pending-acknowledgement field is read for provisional claims

## What

Read a declared field on the peer record that acknowledges a pending claim, so a provisional relation claim (`MILE-154`) is satisfied by the peer's own declaration as well as by the inverse relation.

## Why

A real adopter corpus records the acknowledgement on the peer under a field of its own naming. Without reading it, a provisional claim is reported as unacknowledged even though the peer states it. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

The field name is declared in configuration; a provisional claim whose peer carries the declared acknowledgement is not reported; one whose peer does not still is; a fixture covers both and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
