---
Stable-Id: 01M6JW46TQ0FA01XN38PPQS299
Status: Planned
Phase: '0'
Track: embodiment-model
Implements: SPEC-1
Blocked-on: —
---
# 166 — `drift_grace_days` can count from a declared last-audit field

## What

Let the grace window of `drift_grace_days` (`MILE-129`) count from a declared last-audit date field on the record instead of the file's last change.

## Why

A record's file changes for reasons unrelated to its evidence, so the last change date mis-dates the audit; a real adopter corpus keeps the audit date in a field. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

When a last-audit field is declared and present, the window counts from it; otherwise from the file's last change as today; a malformed date is reported, not ignored; a fixture covers both sources and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
