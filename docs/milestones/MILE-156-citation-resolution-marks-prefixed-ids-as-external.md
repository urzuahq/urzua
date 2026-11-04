---
Stable-Id: 01M6JKNTXSX6T751TK8FXKS9DB
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 156 — Citation.resolution marks prefixed ids as external

## What

Add a way for `citation.resolution` to mark certain prefixed ids as external references (for example a public standard number that shares a record prefix) so they are not resolved as records.

## Why

An external identifier such as `RFC-4122` collides with a record prefix `RFC` and is reported as an unresolvable record citation. There is no way to say the id is not ours, so the author must reword a correct citation to satisfy the checker. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A declared external-id pattern makes a matching citation skip resolution; a non-matching dangling id is still reported; documented with a fixture of a colliding standard number; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
