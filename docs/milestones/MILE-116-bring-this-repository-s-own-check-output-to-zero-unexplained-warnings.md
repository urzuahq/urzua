---
Stable-Id: 01M6CXZT9ANCTR6QP7ZQGAYKQJ
Status: Done
Phase: '0'
Track: governance-process
Realized-by: code:.urzua/config.yaml, code:Makefile, test:.github/workflows/checks.yml
---
# 116 — Bring this repository's own check output to zero unexplained warnings

## What

Bring `urzua check` (the repository gate `make records` runs) on this repository to zero unexplained warnings: each current warning is
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
would otherwise be buried in. Counts were taken by running `urzua check` on this branch (`urzua check docs/` gave the same result).

## Done means

`urzua check` over this repository (what `make records` and `checks.yml` run) reports zero unwaived findings: each warning is fixed at its source or waived by a `waiver` record with a reason and an expiry that `check` itself annotates, `embodiment.locator-promotion-candidate` is off in `.urzua/config.yaml` as an advisory-only rule by maintainer decision, and CI fails when a new finding appears.

## References

- `SPEC-10` -- the `waiver` record type, the sanctioned way to keep a finding visible but non-blocking.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `check docs/` exits 0 with 69 standing warnings across six rules; filed so a new finding on a new record is visible instead of lost among them. | **substantive** |
> | 2026-11-04 | Turned `embodiment.locator-promotion-candidate` off in `.urzua/config.yaml` (advisory only; several ADRs citing one rule module is expected after the MILE-131 split), and applied three record fixes (`MILE-56`, `RFC-39`/`ADR-59`, `BUG-38`). **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #161, #159, #139: `urzua check` over this repository reports zero unwaived findings: each warning is fixed or waived by a waiver record with a reason and an expiry. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
