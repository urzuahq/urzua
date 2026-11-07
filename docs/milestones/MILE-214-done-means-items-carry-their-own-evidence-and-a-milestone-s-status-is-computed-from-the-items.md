---
Stable-Id: 01M6T31G83RF06364RRW04B6JQ
Status: Done
Phase: '0'
Track: schema-governance
Implements: ADR-96
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/done_items.rs, code:rust/crates/urzua-core/src/rules/delivery_items.rs, code:rust/crates/urzua-core/src/rules/status.rs, code:rust/crates/urzua-core/src/status_model.rs, code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-core/src/done_items/tests.rs, test:rust/crates/urzua-cli/tests/done_items_integration.rs, spec:docs/specs/SPEC-6-the-milestone-record-type.md, spec:docs/specs/SPEC-3-configuration.md, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-8-urzua-fix.md
---
# 214 — Done means items carry their own evidence and a milestone's status is computed from the items

## What

Build the engine of `ADR-96`: a type can declare `statuses.delivery.items`, its Done means becomes a list of items that each name their evidence (or a declared waiver), and `judge_delivery`, `status.delivery-agreement`, `status.done-items-shape`, `status.waived-item-stale`, `urzua fix` and `/done` compute the status from the items, with a guaranteed error finding when a record sits at a success status with an unresolved item. This milestone turns nothing on in this repository's config; `MILE-215` migrates the corpus and switches it on.

## Why

`judge_delivery` computes delivered from the evidence tier alone and never reads Done means, so a milestone with one of five items evidenced is forced to `Done` (`MILE-165`, held by `WAIVER-0051`). The maintainer confirmed it as an engine defect on 2026-11-07 and decided evidence per item. The independent review of the same day asked for the engine to land first and the 143-record migration to be its own milestone.

## Done means

- A type that declares `statuses.delivery.items` computes delivered when no item is unresolved and at least one resolves at a `delivered_tiers` tier, partial when some resolve (at least one at a delivered tier), specified-only when every resolved item is at `Specified`, and none when none resolve; a type that declares nothing behaves exactly as before, and a fixture for each outcome, for a spec-only milestone and for the undeclared type fails before the change.
- `status.delivery-agreement` has a `DoneWithUnresolvedItems` variant: a `success` status while any item is unresolved is an error that names each item by number and text and its cause, its severity is a floor that a repository setting cannot lower, a dated `WAIVER` record and the rule's `permanent` categories do not match it (a fixture with a waiver whose `Rule` and `Scope` match still reports it, and the waiver still matches the rule's other variants), and `fix --apply` never downgrades a success status; a fixture shows `check` exit 1 on it under every `fail_on`.
- The variant declares two ADR-73 actions, `edit_field` on the status to the computed partial status (the first non-success, non-failure, non-progress status for a derived-blocked record) and `edit_section` on the Done means item, and not the `Exempt` entry of `delivery-without-evidence`; `urzua explain` prints both and the severity floor.
- `status.delivery-agreement` also reports a `Planned` milestone with some evidenced items (repaired by `fix` to `InProgress` unless it is derived-blocked) and an evidenced-in-full milestone under a not-done status.
- `status.done-items-shape` implements the grammar of `ADR-96` section 1 and reports a prose-only section, a marker in the wrong place, clauses on the first line and on final lines at once, a clause that is not entirely locators (backticks, trailing whitespace text), a leading checkbox, an unknown waiver category, an empty reason, a missing or malformed cite or date; each of the nine worked examples of `ADR-96` is a fixture, and sub-items are never items.
- `status.waived-item-stale` reports a waived item whose evidence clause now resolves, a `deferred-to` whose target is missing or finished, and a waiver past its expiry date; a fixture for each fails before the change, and each waived item is listed in the report by category.
- Config load rejects the invalid `items` declarations listed in `ADR-96` section 4, including `items` on a type whose `status.delivery-agreement` is `off`.
- Item locators are read by `Embodiment`, drift, `embodiment.locator-exists` and `fix --rebaseline`, and a pin inside an item clause is rewritten by a re-pin.
- `/done` on a milestone with three of five items evidenced leaves it `InProgress` and names the unresolved items; `urzua roadmap` reports `items` for an opted-in record and keeps its dependents blocked.
- `SPEC-2`, `SPEC-3`, `SPEC-6` (the grammar, verbatim) and `SPEC-8` document it with revision rows, and a changeset (minor) is added.
- It is exercised end to end by a fixture corpus in the acceptance suite (`SPEC-4`) whose config declares `items`, run through `check`, `fix`, `roadmap` and `explain`; this repository's own config switch is deliberately `MILE-215`'s, not this milestone's.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed. **Why:** maintainer decision of 2026-11-07 (evidence per Done-means item), recorded as `ADR-96`; this milestone builds it and migrates the milestone corpus. | **substantive** |
> | 2026-11-07 | Narrowed to the engine. **Why:** the independent review of `ADR-96` asked that the engine land before the corpus migration, which is `MILE-215`; the Done means gains the guaranteed `DoneWithUnresolvedItems` finding and its actions, the grammar fixtures, the stale check and the `Specified` guard that the review and its answers to the open questions added, and loses the repository switch-on and the 143-record migration. | **substantive** |
> | 2026-11-07 | Cleared `Blocked-on`: ADR-96 is Accepted. **Why:** the wait it named is over. | **structural** |
> | 2026-11-07 | Built and marked Done. `statuses.delivery.items` loads with the section, marker, partial-status, resolving-tier and waiver-category checks of `ADR-96` section 4 (and refuses `items` when `status.delivery-agreement` is off); `status.delivery-agreement` computes delivered, partial, specified-only and none from the items and gains `delivery-partial-not-started`, `delivery-partial-blocked` and the guaranteed `DoneWithUnresolvedItems` error (severity floor and `waivable: false` declared on the rule table's variant, so no setting lowers it and no rule-level waiver or `permanent` category matches it); `status.done-items-shape` implements the grammar with each worked example as a fixture; `status.waived-item-stale` reports the three staleness cases; `fix` computes from the items, never downgrades and writes no `progress` status to a derived-blocked record; item locators are read by `Embodiment`, drift (per clause line), `embodiment.locator-exists` (variant `item-locator-defect`), `fix --rebaseline` and the roadmap, which reports `items`. `SPEC-2`, `SPEC-3`, `SPEC-6` and `SPEC-8` document it. This repository's config does not declare `items`: the switch-on, the `milestone` template's example item line and the migration are `MILE-215`'s. Three readings the record left open: a finding carries one `suggested_action` (`ADR-73`), so `DoneWithUnresolvedItems` carries the reopen (`edit-field`) and the table declares the `edit-section` evidence action as a second variant, which `urzua rules` lists; the floor and `waivable` are printed by `urzua rules`, the command that prints a rule's actions, not by `urzua explain`, which names the records governing a file; and a waiver cite that names a missing record is reported by `status.waived-item-stale`, since an item's cite is not a header pointer the pointer rules read. **Why:** the Done means of this record is met: the engine of `ADR-96` is built and exercised end to end by the fixture corpus in `done_items_integration.rs`, and by a scratch run of this repository's own milestones with the option enabled, whose counts are in the pull request. | **substantive** |
