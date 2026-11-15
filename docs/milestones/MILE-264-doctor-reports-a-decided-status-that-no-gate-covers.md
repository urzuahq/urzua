---
Stable-Id: 01M7FW48GXSSQ2DMC8D1STR7TP
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 264 — doctor reports a decided status that no gate covers

## What

Warn when a record type has a success or implementable status that no authority gate lists, so the gate's list and the type's statuses cannot drift apart unnoticed.

## Why

The adversarial review of the acceptance gate on 2026-11-15 found that a gate names literal statuses and nothing cross-checks them against each type's success statuses: adding an accepted-equivalent status to a type without adding it to the gate leaves it ungated.

## Done means

- `urzua doctor` and config load report a type whose success status is not covered by any status gate, naming the type and the status.
- The report is silent for a type the configuration marks as needing no decider.
- `urzua doctor` reports a clean result on this repository's configuration.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
