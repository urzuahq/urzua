---
Stable-Id: 01M6T40F6S6QC0ZN1F3KQ9Q4W5
Status: Planned
Phase: '0'
Track: schema-governance
Implements: ADR-96
Depends-on: MILE-214
Blocked-on: —
---
# 215 — Migrate the milestone corpus to Done means items and switch evidence per item on

## What

Migrate every `Done` milestone's Done means into the item list of `ADR-96`, give each item its evidence or a declared waiver, reopen any milestone whose claim is not true, and then switch `statuses.delivery.items` on for `milestone` in `.urzua/config.yaml`.

## Why

`MILE-214` builds the engine and leaves it off, so until this lands every `Done` milestone is uncovered and a falsely `Done` one stays unseen. The independent review of `ADR-96` sized the work at 143 hand-split records, most needing new locators, and asked for it to be its own milestone with a reviewer-checked list of reopens and waivers per PR.

## Done means

- Every one of the 143 `Done` milestones carries a Done means list whose items each resolve by evidence or are waived with a declared category, or the milestone is reopened with a revision row naming the unmet item; no blanket waiver is used, and none is kept `Done` for a claim that is not true.
- Each migration PR lists its reopens and its waivers with the item and the reason, and for each `deferred-to` whether the target's own items cover the deferred one, and a reviewer has read each.
- `.urzua/config.yaml` declares `statuses.delivery.items` for `milestone` and no other type, with the four waiver categories of `ADR-96` section 4, in the last PR only, once `urzua check` is clean over every milestone.
- `WAIVER-0051` is removed and `MILE-165`, rewritten into items, computes `InProgress` from the engine.
- `urzua check` and `urzua check --base origin/main` report 0 unwaived findings, including no `DoneWithUnresolvedItems` and no `status.waived-item-stale`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed. **Why:** split out of `MILE-214` on the independent review of `ADR-96`, so the engine can land before the corpus is migrated and switched on. | **substantive** |
> | 2026-11-07 | Cleared `Blocked-on`: `MILE-214` is Done and `ADR-96` is Accepted. **Why:** the wait it named is over; `Depends-on` still orders the two. | **structural** |
