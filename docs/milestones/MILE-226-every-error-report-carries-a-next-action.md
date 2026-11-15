---
Stable-Id: 01M7FVTRXMF3RCQBCSQV6ZJ98Q
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 226 — Every error report carries a next action

## What

Fill `next_action` on the reports of failures a user can fix: an unknown rule or key, a bad path, running outside a git repository, a shallow clone, an unreadable record.

## Why

the audit of 2026-11-15 triggered several errors and `next_action` was null in each. An agent branching on the report has nothing to follow.

## Done means

- An unknown rule id, an unknown config key, a path outside the repository, a run outside a git repository and a shallow clone each return a report whose `next_action` names a command or an edit.
- A test asserts that every could-not-run cause either sets `next_action` or is listed as having none with a reason.
- A load error for a misspelt key suggests the nearest valid key.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
