---
Stable-Id: 01M6K49N8M2J70TKCRYS2HFABF
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/status.rs, code:rust/crates/urzua-core/src/fix.rs, code:rust/crates/urzua-core/src/status_model.rs, test:rust/crates/urzua-cli/tests/status_delivery_integration.rs
---
# 183 — A milestone's Status disagrees with its delivery evidence and no rule reads it

## What

A milestone type can declare how its `Status` is held to delivery evidence. `statuses.delivery` names the computed tiers that mean the work was delivered, the tiers that mean nothing was, the statuses that mean not done, and the success status a delivered record is repaired to. A rule, `status.delivery-agreement`, reports a disagreement both ways, and `urzua fix --apply` writes the success status for the first.

## Why

About thirty Phase 0 milestones were built and merged and still read `Status: Planned`. The milestone type declared no evidence field, so urzua could not tell a built milestone from an unbuilt one, and no rule compared a stated `Status` with delivery evidence in that direction: `embodiment.requires-accepted-status` (`MILE-138`) covers only the opposite one, an unaccepted record claiming to be built. Agents wrote "built in #NNN" in revision-log prose, which no rule reads. Maintainer decision of 2026-11-04: the change that delivers a milestone sets its `Realized-by` and `Status: Done` itself, and CI fails when they disagree.

## Done means

A milestone whose `Realized-by` (or a citing source back-pointer) computes to a declared delivered tier while its `Status` is a declared not-done status is an error, and `urzua fix --apply` sets the declared success status with a structural revision row; a `Done` milestone whose evidence computes to a declared unevidenced tier is an error that `fix` reports and does not downgrade; the mapping is declared per type and the engine hardcodes no status, tier set or type; this repository declares it on `milestone`, every milestone states a `Status` its evidence agrees with, and `urzua check` is clean over them.

> **Revision log**
>
> | Date | Change | Class |
> | 2026-11-04 | Built and delivered: `statuses.delivery` declares the evidence-to-status mapping per type, `status.delivery-agreement` reports both directions of disagreement, and `urzua fix --apply` writes the success status for a delivered record (`SPEC-2`, `SPEC-3`, `SPEC-8`). `Realized-by` and `Status: Done` set by the delivering change itself. **Why:** maintainer decision of 2026-11-04: the change that delivers a milestone sets its evidence and its `Done`, and CI fails when they disagree. | **structural** |
> |---|---|---|
