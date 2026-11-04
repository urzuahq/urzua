---
Stable-Id: 01M6JKNVFM208DS7MJ5A0TAMEE
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 157 — Cross-field date ordering

## What

Add a rule that checks one date field is not earlier than another in the same record (for example an acceptance date earlier than the record's own date).

## Why

A real adopter checks that a record is not accepted before it was written. urzua has no cross-field date check, so a verifiably impossible chronology passes; the dates are already declared fields, so the check needs no new evidence. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A configured pair of date fields is compared; an out-of-order pair is reported; equal dates pass; an absent or unparseable date is left to the existing field-quality rules; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
