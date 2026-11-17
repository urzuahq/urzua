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

On release 0.4.7 `urzua init` writes `authority: {decider_table: base, waivable: never}` and `harness: {outside_writes: shared}`: no approver group, no gate, no adapter, no guardrail. The block looks like protection and does nothing. In an adopting repository an agent wrote eight decision records straight to their accepted status; `urzua check`, `urzua doctor`, the git hooks and CI passed every time. `doctor`'s line says no status move or path is gated until a gate is added, at status ok, which no one reads as a warning. Setting it up afterwards showed three more silent gaps: `diff.decider-required` at `error` with no actors file reports at info and CI passes, unless `require_evidence` is set; the CI action's `actors` input is off by default and nothing notices; and CI compares only the pull request author's login, so where agent work is pushed under the maintainer's own credentials the CI check passes whatever the agent did, which nothing states. The adopting agent reported on 2026-11-17 that a session rooted in another directory loaded no hook and could still move a record to its accepted status, that `urzua harness install` listed an item with an empty path and state `not-configured`, and that every allowed edit carried a "did NOT judge this edit" message. For a tool whose purpose is to stop a record being overridden, the adopting agent assumed the opposite default.

## Done means

- Building on `MILE-264`, which reports a type whose success status no gate covers, `urzua doctor` reports it at warn, not at status ok, naming the type, the status and the keys to add, shown by a test with a freshly initialised configuration.
- `urzua doctor` warns when a gate is declared and no installed harness guardrail enforces it locally, and when `diff.decider-required` is enabled without `require_evidence` or the workflow that runs the gate does not pass who made the change, shown by a test of each.
- `urzua init` ends its report with what it did not set up (approver groups, gates, the harness) and the next action for each, and the adoption guide has a section with a tested configuration for reserving a status to named people, the install command, the files it writes, and the steps by which a person accepts a record.
- The reference states plainly what the CI rule proves and does not: it compares the pull request author or a declared approver with the group, so it binds agent work only when that work is not pushed under an approver's own credentials; and that an edit made through the shell is reported after the fact, not prevented.
- The reference and `urzua harness install`'s report state that the harness binds an agent session opened in the repository, not the repository: a session rooted in another directory that edits these files loads no hook.
- `urzua harness install` names what it skipped and the key that would enable it, and, until the harness no longer installs a script (`MILE-339`), tells the reader to exclude the installed harness files from a formatter, since the settings command pins the script's hash; and an allowed edit prints nothing about a guardrail the configuration turned off by choice, where today every allowed edit carries a "did NOT judge this edit" message that teaches agents to ignore the harness, shown by a test of each.
- A rule set to error that cannot run for lack of its input reports that at the rule's level where the input can exist, and says so without blocking where it cannot: in CI a missing record of who made the change is an error; in a local git hook, where no such record can exist, it is reported and the commit proceeds. On 2026-11-17 the setting that makes the missing input an error in CI (`require_evidence`) stopped every local commit of a gated move in an adopting repository, a person's included, leaving `--no-verify` as the only way through. Shown by a test of each place.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository on release 0.4.7, at the maintainer's prompting: the maintainer noticed that agent had written a decision record straight to its accepted status and asked what stopped that in this repository and not in that one. | **structural** |
