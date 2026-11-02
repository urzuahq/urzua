---
Stable-Id: 01M6CXZT9ANCTR6QP7ZQGAYKQJ
Status: Planned
Phase: '0'
Track: governance-process
---
# 116 — Bring this repository's own check output to zero unexplained warnings

## What

Bring `urzua check docs/` on this repository to zero unexplained warnings: each current warning is
fixed at its source, or waived through a `waiver` record with a reason and an expiry, so `check`
itself annotates it (`SPEC-10`). A warning merely mentioned by another record does not count: it stays
indistinguishable from a new finding in `check`'s output. The count today is 69, all `warn`-level, by rule: `embodiment.consistency` 34,
`embodiment.locator-promotion-candidate` 18, `pointer.target-status` 6, `field.pending` 5,
`narrative-field.stale` 5, `header.field-set-consistency` 1.

Also in scope: inconsistencies between records and the specs that define their type that no rule
reports yet. Known today: `SPEC-6` defines a milestone's `Implements` as RFC/ADR lineage, but
`MILE-100` to `MILE-104` point it at specs; either the spec or those records changes.

## Why

A bootstrap corpus that carries standing warnings teaches everyone, agents included, to ignore the
output of the tool whose whole job is to be believed. The warnings exit 0, so nothing forces a
decision about them, and a new finding on a new record is indistinguishable from the 69 already there.
`AGENTS.md` already says a live finding is signal ("Don't patch around a finding on your own new
work"); this milestone is the work of making that rule cheap to follow by clearing the backlog it
would otherwise be buried in. Counts were taken by running `urzua check docs/` on this branch.

## References

- `SPEC-10` -- the `waiver` record type, the sanctioned way to keep a finding visible but non-blocking.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `check docs/` exits 0 with 69 standing warnings across six rules; filed so a new finding on a new record is visible instead of lost among them. | **substantive** |
