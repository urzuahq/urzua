---
Stable-Id: 01M7FW6NMTNNH708HAJ959A373
Status: Planned
Phase: '1'
Track: embodiment-model
Blocked-on: null
---
# 269 — A shallow clone is not reported as ok

## What

Make a local run on a shallow clone that cannot judge drift report that, not a green result with a notice.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary. On a shallow clone `check` returned ok with exit 0 and a notice that a result of no drift cannot be trusted; only the gate action refuses a shallow checkout.

## Done means

- On a shallow clone, with a drift rule enabled, `check` reports the drift rules as not run and its status is not ok.
- The report's next action is the command that fetches the missing history.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
