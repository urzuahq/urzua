---
Stable-Id: 01M7FW0SQTAQEM4RJ6SSMTA8ZQ
Status: Planned
Phase: '2'
Track: accountability-identity
Blocked-on: null
---
# 251 — Authority gates take a count, a source state and teams

## What

Extend an authority gate with a minimum number of approvers, the state a move comes from, and teams resolved from the actors file; and gate who may renew a claim.

## Why

the audit of 2026-11-15's extensibility experiments found that a gate is any-one-of a list of logins: two of N cannot be expressed, a gate cannot depend on the source state, a team is a token repeated in two places, and `reverify --by` accepted a name with no authority.

## Done means

- A decision record amends the authority model of `ADR-89`.
- A gate with a minimum of two fails with one approver and passes with two, shown by a test.
- A gate that names a source state applies only to moves from it.
- A team name in a gate is resolved from the actors file, and an unresolved team is reported.
- A renewal by a person the configuration does not allow is refused.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
