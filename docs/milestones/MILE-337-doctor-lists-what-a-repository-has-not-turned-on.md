---
Stable-Id: 01M7KSGSQZMAS6K264ZQK4SNHY
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 337 — doctor lists what a repository has not turned on

## What

Have `urzua doctor` report, beside configuration health, each capability the configuration does not use: one line on what it prevents and the block or command that enables it, so that ok means configured as chosen, not merely loadable.

## Why

In one day an adopting repository on releases 0.4.6 and 0.4.7 adopted eight capabilities, each only after a mistake or a prompt and each from a block handed to it already tested: declared field values, reference target types, requirements keyed on a field, examining untracked records, a checklist that computes the finished status, the field for a replaced record, gates on who may accept, and the agent harness. Its agent ran `urzua init`, saw `doctor` ok and `check` ok, installed the git hooks because `doctor` asked, and started work. An agent follows `next_action`; no next action ever said that acceptance was ungated, that no harness was installed, or that the finished status could be computed from the record's checks. The knowledge exists in the specifications and in this repository's own configuration; nothing in the tool's output leads to it.

## Done means

- `urzua doctor` has a section listing each capability the configuration does not declare, each with what it prevents in one sentence and a `next_action` that is the command or the smallest configuration block to enable it, shown by a test on a freshly initialised configuration that names at least: gates on who may move a record to a success status, the agent harness, a checklist that computes status, declared field values, reference target types, and examining untracked records.
- The list is derived from the configuration schema and the rule table, so a capability added to the engine appears without the list being edited by hand, shown by a test that fails when a capability has no entry.
- A repository records a capability it has decided not to use, with a reason, and `doctor` then stops listing it; `doctor` reports ok only when every capability is either declared or declined.
- After an upgrade, `doctor` names the capabilities the new release added that the configuration neither declares nor declines.
- The section names no record type, status or field of its own: its examples use the repository's own declared names.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Proposed on 2026-11-17 by the maintainer, relayed by an agent in an adopting repository: "I feel like we've missed a bunch of things during setup that we probably should have turned on. Is this a documentation issue?", with the proposal that the tool supply skills that "tell the agent all the things that you're missing ... like a guided setup with the user". | **structural** |
