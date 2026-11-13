---
Status: Planned
Stable-Id: 01M1Y5JW41FC70SRJP18K5C4XX
Phase: '2'
Track: escalation
Implements: RFC-14
Depends-on: COND-4
Blocked-on: COND-4 -- a decision to actually build it; its Signal says exactly what counts as met.
---
# 30 — Forge app

## What

A collector that needs no write access, per RFC-14's shape.

## Why

ADR-25 already decided the shape is real but explicitly not a build commitment.

## Done means

- A forge app collects records through the forge without write access.
- A test asserts the credentials it asks for are read-only.
- It is built only once the maintainer decides to build it (`ADR-25` made the shape real but no build commitment), and that decision is cited here.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Status` Blocked -> Planned; `Blocked-on` unchanged. **Why:** `Blocked` is derived only (`RFC-48`, `ADR-83`) and this repository no longer writes it as a status. Nothing has started, so the status that matches the progress is `Planned`. What it waits on (a decision to actually build it) is a fact or a decision, not a record, so there is no `Depends-on` to state; `Blocked-on` keeps the wait until the `condition` record type exists (`RFC-48`). | **substantive** |
> | 2026-11-05 | `Depends-on: COND-4` added; `Blocked-on` now an explanatory note. `Status` unchanged. **Why:** `MILE-187`: the wait on a decision to build the forge app was prose, invisible to `urzua roadmap` and the derived `blocked` value; it is now one `Open` condition, COND-4, whose `Signal` states what counts as met. A person moves the condition to `Met`, not this change. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
