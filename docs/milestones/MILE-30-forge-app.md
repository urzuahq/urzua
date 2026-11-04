---
Status: Blocked
Stable-Id: 01M1Y5JW41FC70SRJP18K5C4XX
Phase: '2'
Track: escalation
Implements: RFC-14
Blocked-on: a decision to actually build it
---
# 30 — Forge app

## What

A collector that needs no write access, per RFC-14's shape.

## Why

ADR-25 already decided the shape is real but explicitly not a build commitment.

## Done means

A forge app collects records through the forge without write access, and a test asserts the credentials it asks for are read-only; it is built only once the maintainer decides to build it (`ADR-25` made the shape real but no build commitment), and that decision is cited here.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |