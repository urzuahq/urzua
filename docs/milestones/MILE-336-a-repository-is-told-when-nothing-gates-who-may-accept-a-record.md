---
Stable-Id: 01M7KSDGJKDFSR4VRTJ6C9BX9F
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 336 — A repository is told when nothing gates who may accept a record

## What

Make an ungated authority setup visible: `urzua init` says what it did not set up, and `urzua doctor` warns when a status a type declares as its success or implementable state can be reached by anyone, when the rule that enforces gates cannot see who made a change, and when a gate is declared with no harness to enforce it locally.

## Why

On release 0.4.7 `urzua init` writes `authority: {decider_table: base, waivable: never}` and `harness: {outside_writes: shared}`: no approver group, no gate, no adapter, no guardrail. The block looks like protection and does nothing. In an adopting repository an agent wrote eight decision records straight to their accepted status; `urzua check`, `urzua doctor`, the git hooks and CI passed every time. `doctor`'s line says no status move or path is gated until a gate is added, at status ok, which no one reads as a warning. Setting it up afterwards showed three more silent gaps: `diff.decider-required` at `error` with no actors file reports at info and CI passes, unless `require_evidence` is set; the CI action's `actors` input is off by default and nothing notices; and CI compares only the pull request author's login, so where agent work is pushed under the maintainer's own credentials the CI check passes whatever the agent did, which nothing states. For a tool whose purpose is to stop a record being overridden, an adopter assumes the opposite default.

## Done means

- `urzua doctor` warns, not notes, when a type declares a success or implementable status and no gate covers the move to it, naming the type and status and the keys to add, shown by a test with a freshly initialised configuration.
- `urzua doctor` warns when a gate is declared and no installed harness guardrail enforces it locally, and when `diff.decider-required` is enabled without `require_evidence` or the workflow that runs the gate does not pass who made the change, shown by a test of each.
- `urzua init` ends its report with what it did not set up (approver groups, gates, the harness) and the next action for each, and the adoption guide has a section with a tested configuration for reserving a status to named people, the install command, the files it writes, and the steps by which a person accepts a record.
- The reference states plainly what the CI rule proves and does not: it compares the pull request author or a declared approver with the group, so it binds agent work only when that work is not pushed under an approver's own credentials; and that an edit made through the shell is reported after the fact, not prevented.
- A rule set to error whose evidence is missing never passes silently: the missing evidence is reported at the rule's level unless the configuration says otherwise.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository on release 0.4.7, at the maintainer's prompting: the maintainer noticed that agent had written a decision record straight to its accepted status and asked what stopped that in this repository and not in that one. | **structural** |
