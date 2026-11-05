---
Stable-Id: 01M6MKSTD2091W0RSX4TQJ8XKT
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-7
---
# 78 — RFC-7's remaining enforcement-harness decisions

## Context

`ADR-69`, `ADR-70` and `ADR-71` record the parts of `RFC-7` that Phase 0 builds: the pre-write hook that
relays `urzua`'s verdict, drafting commands, and the layered gate. They left `RFC-7`'s remaining
questions open, and `MILE-12` cannot close while a part of `RFC-7` is neither decided nor deferred with
an owner. The maintainer decided the remainder on 2026-11-05. What is true today:

- **The hook's override is visible only in its output.** `scripts/hooks/record-edit-hook.sh` accepts a
  named override and prints it, then discards it (`persist_override` is an empty function). `ADR-69`'s
  Consequences say that without persistence the override is an audit trail only for as long as the
  transcript lives.
- **The hook judges one computed field.** It relays `urzua fix --stdin-path`, which reports `Embodiment`
  repairs only. A header, field, section or status error in the same edit passes the editor and is
  caught at the commit and in CI (`ADR-71`).
- **`RFC-7` section 4** (scope-mismatch between a branch's stated purpose and its diff) already has a
  rule: `diff.source-and-records-mixed` (`MILE-142`), opt-in, reporting `not-applicable` on a run with
  no `--base`. This repository does not enable it, because it deliberately changes records and code in
  one pull request.
- **`RFC-7` section 5** (prefer the lighter mechanism) has no check. A `Realized-by` locator on a file
  under `back_pointer_paths` is accepted although an `Implements:` or `Verifies:` comment there is
  scanned automatically.
- **A skipped workflow stage** (a decision shipped with no RFC, a change with no ADR) has a record type
  for exactly this kind of recorded exception: the waiver (`ADR-11`).
- **A hook-blocked edit** never reaches the file, so there is nothing to append a revision row to
  (`ADR-14`, `RFC-6`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave the six questions on `RFC-7` | Nothing to build | `MILE-12` stays open; the override stays an audit trail only as long as a transcript |
| B. A notification or routing system for overrides | Someone is told | A service with its own policy and owner; nothing in this repository needs it |
| C. Answer each question with what the tool already has: a committed log read by `check`, one `check` call at the hook, an existing rule, one new opt-in warning, an existing record type (chosen) | No new substrate; every answer is a declared, checkable thing | The hook needs `check --stdin-path`; one rule is new |
| D. A new skip-declaration field for workflow stages | Names the exception exactly | A second exception mechanism beside the waiver, with no computable ground truth to disagree with |

## Decision

In the context of `RFC-7`'s enforcement harness being partly built (`ADR-69` to `ADR-71`), facing six
questions it left open, we decided **the six answers below, each reusing a mechanism the tool already
has**, to achieve an enforcement harness whose overrides are recorded and reviewed and whose editor
layer judges the whole record, accepting that an override is surfaced to a reviewer and not routed to
anyone.

1. **A hook override is recorded in a committed override log.** Each entry carries the override's name,
   the field, the record, the reason and the date. `check` reports an override that has not yet been
   reviewed, so it surfaces in the pull request that carries it. There is no notification or routing
   system: the pull request is the channel. This answers `ADR-69`'s open questions on where an override
   is persisted and whom an unacknowledged block routes to.
2. **The pre-write hook judges every rule on the edited record, not only computed fields.** This needs
   `urzua check --stdin-path`, the prospective-content form of `check` that `fix --stdin-path` is for
   `fix`. An error blocks the edit; a warning is shown and does not. The `ADR-69` behaviour is kept:
   when `urzua` cannot judge, the edit proceeds and the hook says loudly that it did not judge, and a
   block can be passed only with a named override that appears in the output and is recorded by item 1.
   This answers `ADR-69`'s open question on whether other computed fields join `Embodiment`: all of the
   record's rules do.
3. **`RFC-7` section 4 is satisfied by the existing opt-in `diff.source-and-records-mixed`.** No new
   check is built. It stays off in this repository, because this repository deliberately mixes records
   and code in one change; an adopter that wants the prompt enables it.
4. **`RFC-7` section 5 becomes an opt-in warning rule.** For a `Realized-by` locator that names a file
   under `back_pointer_paths`, it warns and nudges toward an `Implements:` or `Verifies:` comment in that
   file. It is a nudge and not an error, because the two mechanisms are not interchangeable in general:
   `Realized-by` is the only option for a path the comment scan cannot reach.
5. **Skipping a workflow stage is expressed with an existing waiver record.** There is no new skip
   mechanism and no `Skip-RFC` field.
6. **A hook-blocked edit leaves no revision record.** This matches `RFC-7`'s lean: a blocked edit never
   happens, as a failed compile leaves no artifact. The override log of item 1 records an edit that was
   let through, not one that was stopped.

With these, every part of `RFC-7` is decided in an ADR or deferred by name, and `MILE-12` closes when
this ADR is accepted.

## Reversibility

Easy for items 3, 4, 5 and 6, which add no data and one opt-in rule. Items 1 and 2 are the commitments: a
committed override log becomes history a reader may cite, and a hook that judges every rule can be
switched off but not made to have judged earlier edits. The cost of being wrong is a hook that blocks too
much, which is overridden often (and recorded as such by item 1), or a log nobody reviews.

## Consequences

- An override stops being an audit trail only for the life of a transcript: it is a committed entry a
  reviewer sees in the pull request.
- `urzua check` gains `--stdin-path` and a finding for an unreviewed override, and the override log
  needs a declared location and shape. `SPEC-2` and `SPEC-8` change when this is built.
- The editor layer now blocks on a header, field, section or status error in the edited record, so the
  hook is only as fast as a single-record `check` run, and the same loud "not judged" output is needed
  for every way `check` cannot run.
- The new rule enters the rule table (`make rule-table-check`) and `SPEC-2`.
- An unreviewed override with no one assigned to review it can sit in the log. The pull request gate
  surfaces it; it does not guarantee a person acts.
- `RFC-7` stays a Draft whose parts are now each decided in `ADR-69` to `ADR-71` or here.
- The milestones that build items 1, 2 and 4 are not filed by this decision.

## References

- `RFC-7` -- the enforcement harness; sections 2 to 6 and its open questions.
- `ADR-69`, `ADR-70`, `ADR-71` -- the parts already decided, which this builds on without changing.
- `ADR-11` -- the waiver record that expresses a skipped stage.
- `ADR-14` -- the append-only revision log, which a blocked edit never reaches.
- `MILE-12` -- closes when this ADR is accepted.
- `MILE-142` -- `diff.source-and-records-mixed`.
- `SPEC-2`, `SPEC-8` -- `check` and `fix`, where `--stdin-path` is specified.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05, recorded here; the maintainer accepts it in a later change. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
