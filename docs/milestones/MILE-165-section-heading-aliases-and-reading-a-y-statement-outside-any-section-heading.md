---
Stable-Id: 01M6JW46AMFZNJ0MNECN7Z4VJ1
Status: Planned
Phase: '0'
Track: section-checks
Implements: SPEC-1
Blocked-on: —
---
# 165 — Section heading aliases, and reading a Y-statement outside any section heading

## What

Accept declared aliases for a section's heading, and read a Y-statement that sits outside any `##` section.

## Why

Real adopter records name the same section differently and sometimes carry the Y-statement in the preamble, so a single literal heading and a section-only reading produce false errors. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A declared alias satisfies the section requirement and is reported as the alias used; a Y-statement found before the first `##` heading is judged by the same shape check; a fixture covers each and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
