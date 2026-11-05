---
Status: Planned
Stable-Id: 01M1Y5JJZAAG3F4D703671BK2C
Phase: '2'
Track: governance-process
Implements: RFC-9
Blocked-on: RFC-9's own Q2
---
# 13 — Decide RFC-9: agentic repair tier

## What

An unreliable generator behind a mechanical verifier.

## Why

Explicitly deferred by the project lead: resolve Q2 first.

## Done means

`RFC-9` question 2 is resolved by the maintainer and recorded; the agentic repair tier is then either decided in an ADR (an unreliable generator behind a mechanical verifier, with the verifier's acceptance as the only write path) or rejected with the reason, and this record's `Blocked-on` is cleared by that decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Status` Blocked -> Planned; `Blocked-on` unchanged. **Why:** `Blocked` is derived only (`RFC-48`, `ADR-83`) and this repository no longer writes it as a status. Nothing has started, so the status that matches the progress is `Planned`. What it waits on (RFC-9's own Q2) is a fact or a decision, not a record, so there is no `Depends-on` to state; `Blocked-on` keeps the wait until the `condition` record type exists (`RFC-48`). | **substantive** |
