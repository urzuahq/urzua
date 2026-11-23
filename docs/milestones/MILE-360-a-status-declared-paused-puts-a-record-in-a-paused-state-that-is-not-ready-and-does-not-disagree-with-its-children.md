---
Stable-Id: 01M83AV7FRY1A2YV05YAYTPJPR
Status: Planned
Phase: '1'
Track: roadmap-tracking
Blocked-on:
---
# 360 — A status declared paused puts a record in a paused state that is not ready and does not disagree with its children

## What

A record type may declare `statuses.paused`: the non-terminal statuses a person writes to park work
on purpose. A record in one of them gets the export state `paused`, is not `ready`, is left off
`urzua roadmap`'s ready list and the board's Ready column, and as a parent is not reported by
`state_disagrees_with_children` or `status.children-agreement` for having finished or started
children.

## Why

A consuming repository parks work with `Paused`, and urzua 0.4.13 cannot say so. With `Paused` in
`statuses.values` and in no role, the export gives the record `state: not-started` and
`ready: true`, so `urzua roadmap` and the board present parked work as ready to an agent reading
them; a paused parent with a finished child is reported as disagreeing with it, and no
configuration clears that. The repository works around it by skipping paused records in its own
check.

`statuses.blocked` does not fit. It names a status a person writes to say a record waits on another
record (`ADR-83`, `RFC-48`); `sequence.status-agreement` reports a record in a `blocked` status
that nothing blocks, so a parked record listed there is reported as wrong, and the export's state
and `ready` do not read `blocked` at all. A paused status is the opposite case: nothing blocks the
record, a person chose to stop it. It belongs beside `success`, `failure`, `progress` and `blocked`
as one more declared role, read from configuration, not a status name in Rust.

## Done means

- `statuses.paused` is validated at load: a subset of `values`, disjoint from `success`, `failure`, `progress` and `blocked`, with each violation a load error that names the status.
- A record in a declared paused status exports `state: paused` and `ready: false`, shown by a test with the same record reported `not-started` and `ready: true` when the role is not declared.
- `urzua roadmap` leaves a paused record off its ready list and the board shows it outside Ready, each shown by a test.
- A paused parent with a finished or started child sets neither `state_disagrees_with_children` nor a `status.children-agreement` finding, while a paused child under a finished parent is still counted open, each shown by a test.
- `SPEC-3` documents `statuses.paused` and states that the export's state and `ready` do not read `statuses.blocked`; the export reference lists the `paused` state; each spec carries a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Filed as planned work from a consuming repository's report against urzua 0.4.13, relayed by the org coordination thread; nothing built, no evidence yet. | **structural** |
