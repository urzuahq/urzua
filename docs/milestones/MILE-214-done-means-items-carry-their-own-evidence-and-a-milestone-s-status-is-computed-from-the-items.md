---
Stable-Id: 01M6T31G83RF06364RRW04B6JQ
Status: Planned
Phase: '0'
Track: schema-governance
Implements: ADR-96
Blocked-on: ADR-96 is Proposed; build after the maintainer accepts it.
---
# 214 — Done means items carry their own evidence and a milestone's status is computed from the items

## What

Build `ADR-96`: a type can declare `statuses.delivery.items`, its Done means becomes a list of items that each name their evidence (or a declared waiver), and `judge_delivery`, `status.delivery-agreement`, `urzua fix` and `/done` compute the status from the items. This repository turns it on for `milestone` and migrates every milestone's Done means.

## Why

`judge_delivery` computes delivered from the evidence tier alone and never reads Done means, so a milestone with one of five items evidenced is forced to `Done` (`MILE-165`, held by `WAIVER-0051`). The maintainer confirmed it as an engine defect on 2026-11-07 and decided evidence per item.

## Done means

- A type that declares `statuses.delivery.items` computes delivered only when every non-waived item resolves, partial when some do, none when none do; a type that declares nothing behaves exactly as before, and a fixture for each outcome and for the undeclared type fails before the change.
- `status.delivery-agreement` reports a `success` status with unresolved items by item number and text, a `Planned` milestone with some evidenced items (repaired by `fix` to `InProgress` unless it is derived-blocked), and an evidenced-in-full milestone under a not-done status; `fix --apply` never downgrades a success status.
- `status.done-items-shape` reports a prose-only section, an unparseable clause, a leading checkbox, an unknown waiver category, an empty waiver reason and a missing citation; config load rejects the invalid `items` declarations listed in `ADR-96`.
- Item locators are read by `Embodiment`, drift, `embodiment.locator-exists` and `fix --rebaseline`, and a pin inside an item clause is rewritten by a re-pin.
- `/done` on a milestone with three of five items evidenced leaves it `InProgress` and names the unresolved items; `urzua roadmap` reports `items` for an opted-in record and keeps its dependents blocked.
- `SPEC-2`, `SPEC-3`, `SPEC-6` and `SPEC-8` document it with revision rows, and a changeset (minor) is added.
- It is exercised in this repository: `.urzua/config.yaml` declares `items` for `milestone`; every one of the 143 `Done` milestones carries items, each resolved by evidence or waived with a category, or is reopened with a revision row naming the unmet item; `WAIVER-0051` is removed and `MILE-165` computes `InProgress` from the engine; `urzua check` and `urzua check --base origin/main` report 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed. **Why:** maintainer decision of 2026-11-07 (evidence per Done-means item), recorded as `ADR-96`; this milestone builds it and migrates the milestone corpus. | **substantive** |
