---
Stable-Id: 01M7HW3F4KMGT8EHJEXTTCB6PG
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 299 — Builders do not stall on a protected path the change needs

## What

Give an agent a way to propose a change to a protected path that a decider approves in place, so a feature that needs one line of configuration does not wait for a person to type it.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Five pull requests each needed a one-line change to the configuration or a workflow file; the harness rightly refused the agent, and each change then waited for the orchestrator or maintainer to make it by hand through the shell.

## Done means

- An agent's refused edit to a protected path is recorded as a proposed change that a decider can approve in the conversation, after which that one edit is applied, shown by a test.
- A proposed change the decider declines is discarded and logged.
- The mechanism reuses the approval of `ADR-102` and adds no second way to approve.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
