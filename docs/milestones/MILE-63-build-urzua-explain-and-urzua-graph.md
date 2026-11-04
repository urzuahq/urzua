---
Status: Done
Stable-Id: 01M1YNND3FMY2F2A0N3K0FVT3H
Phase: '0'
Track: governance-process
Implements: ADR-24
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/explain.rs, code:rust/crates/urzua-cli/src/commands/graph.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 63 — Build urzua explain and urzua graph

## What

`urzua explain <path>`: every record whose `Realized-by` names that file as evidence. `urzua graph`: the full `Implements`/`Derives-from`/`Supersedes` relationship graph as data, flagging edges that don't resolve.

## Why

The record graph existed in the schema from the start but nothing surfaced it as queryable data -- `explain` answers "which decisions govern this file," the highest-frequency read an agent or reviewer actually has.

## Done means

`urzua explain <path>` lists every record whose `Realized-by` names that file, and `urzua graph` emits the `Implements`/`Derives-from`/`Supersedes` graph as data with unresolved edges flagged; both are read-only, covered in `check_integration.rs`, and documented in `SPEC-13`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
