---
Stable-Id: 01M7FW3GW0P3TFPQB8CEE61M4D
Status: Planned
Phase: '1'
Track: embodiment-model
Blocked-on: null
---
# 261 — Accepted ADRs state their embodiment

## What

Give every Accepted ADR a truthful embodiment: evidence where it is implemented, and an explicit statement where it needs none.

## Why

the audit of 2026-11-15 found 52 of 94 Accepted ADRs with no embodiment, a null one or `Not started`, including decisions the code has implemented for months.

## Done means

- No Accepted ADR has a null or missing embodiment.
- An ADR that code implements cites it as evidence, and `urzua fix` computes its state.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
