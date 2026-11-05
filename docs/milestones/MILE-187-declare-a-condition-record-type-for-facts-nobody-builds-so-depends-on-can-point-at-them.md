---
Stable-Id: 01M6N53TQASB8WHQTZ217WHJ5W
Status: Done
Realized-by: code:.urzua/config.yaml, spec:docs/specs/SPEC-23-the-condition-record-type.md, test:rust/crates/urzua-cli/tests/condition_type_integration.rs
Phase: '0'
Track: roadmap-tracking
Implements: RFC-48
Depends-on: MILE-182
Blocked-on: —
---
# 187 — Declare a condition record type for facts nobody builds, so Depends-on can point at them

## What

A `condition` record type, declared entirely in `.urzua/config.yaml`, for a dependency on a fact that is observed rather than built: an open question, or a signal such as an adopter asking for a feature. A condition has a status of `Open`, `Met` or `Dropped`; a `Met` condition names its evidence in `Met-by`. A milestone, ADR or RFC points at one with `Depends-on`, as `RFC-48` ("Conditions: dependencies on facts nobody builds") proposes, so a wait on a fact is as structured as a wait on work.

## Why

`RFC-48` (Accepted) sorts blockers into work, which a `Depends-on` link to a record expresses, and facts, which no record expresses today. Four milestones wait on a fact written as prose in `Blocked-on` (`MILE-13`, `MILE-21`, `MILE-23`, `MILE-30`), and `sequence.unresolved-blocker` rightly reports prose in `Depends-on`, so these waits are invisible to `urzua roadmap` (`MILE-182`) and to the derived `blocked` value (`MILE-144`). Maintainer decision of 2026-11-05: file the condition type as a Phase 0 milestone so the roadmap can be computed from structured waits. The type uses only primitives that exist (a declared directory, prefix and identity, a status vocabulary with success and failure sets, `transitions`, and `statuses.require`), which is the test `RFC-48` sets for it: the first record type composed entirely from configuration.

## Done means

- `.urzua/config.yaml` declares a `condition` record type (`dir: docs/conditions`, prefix `COND`, padded filenames `COND-NNNN-slug.md` per `ADR-36`, YAML frontmatter). Its statuses are `Open`, `Met` and `Dropped`; `Open` is the only initial state and moves to `Met` or `Dropped`, both terminal; `Met` is the success state and `Dropped` the failure state; `Met-by` is required in `Met`. The engine gains no special case for the name `condition`: the type is read from configuration alone, and a test over a fixture corpus that declares the same shape under another type name behaves identically.
- A new SPEC documents the record type (header, statuses, the `Signal` section, how `Met-by` is used) and `SPEC-3` records that `condition` is declared in configuration. The type's `spec:` key names it, so `type.no-declared-spec` stays quiet.
- `Depends-on` on a milestone, ADR or RFC resolves to a condition record: `sequence.cycle` and `sequence.unresolved-blocker` examine the edge (a fixture with a dependency on an `Open` condition, a `Met` one and a missing one), and a pair whose target type declares no status is reported rather than read as satisfied (`ADR-60`).
- `urzua roadmap` (`MILE-182`) and the derived `blocked` value (`MILE-144`) read a condition as any other record: a dependent of an `Open` condition is blocked and out of the ready set, a dependent of a `Met` one is not, and a dependent of a `Dropped` one stays out of the ready set. This is why this milestone `Depends-on` `MILE-182`: the roadmap reads the type, and it is the fixture and the real-corpus run that prove it.
- `MILE-13`, `MILE-21`, `MILE-23` and `MILE-30` migrate their prose waits to condition records, one `Open` condition each, with a `Signal` that says exactly what counts as met, and a `Depends-on` from the waiting milestone. `Blocked-on` keeps an explanatory note. A person, not this milestone, moves any of them to `Met`. `urzua check` reports no unwaived finding for the new records, and `sequence.unresolved-blocker` has no prose left to report on these four.
- Not in scope, owned by a separate decision: `condition.review-due` and the `Review-by` field, and `sequence.failed-dependency`. Each is a new rule `RFC-48` proposes and no accepted ADR covers, so building either needs its own ADR first, as `ADR-76` did for the two sequence rules and `ADR-65` for the relation pairs. Declaring the type, its statuses, the `Met-by` requirement and the migration add no engine behaviour and need no ADR of their own; `ADR-72` (transitions) and `ADR-60` (cross-record status reads) already cover the primitives. If building shows a primitive is missing, stop and draft an ADR instead of widening the engine here.
- One tension to settle when migrating: `RFC-48` says a prose blocker that names a decision (such as `MILE-13`'s "RFC-9's own Q2" or `MILE-30`'s "a decision to actually build it") points at the RFC or a decide-milestone instead of a condition. The maintainer's decision of 2026-11-05 names all four milestones for migration, so each becomes a condition whose `Signal` cites the decision, and the condition's `Met-by` is the record that records it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** Maintainer decision, 2026-11-05: the `condition` record type of `RFC-48` is a Phase 0 milestone, so waits on facts are structured and the computed roadmap sees them. | **substantive** |
> | 2026-11-05 | `Blocked-on` cleared. **Why:** `MILE-182` is Done, so `urzua roadmap` exists to read the type; the dependency stays declared in `Depends-on`. `Status` unchanged. | **structural** |
> | 2026-11-05 | `Status` Planned -> Done; `Realized-by` set. **Why:** delivered in full by this change: the `condition` type declared in `.urzua/config.yaml` with `SPEC-23` and the `SPEC-3` row, the fixtures in `condition_type_integration.rs` (same shape under another name; `Depends-on` on `Open`, `Met`, `Dropped` and missing conditions; the roadmap and derived `blocked`), and `MILE-13`, `MILE-21`, `MILE-23` and `MILE-30` migrated to `COND-1` to `COND-4`. `condition.review-due` and `sequence.failed-dependency` stay out of scope, as the record says. | **structural** |
