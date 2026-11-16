---
Stable-Id: 01M7GY7PG2WDWVMDZN5Q84CABH
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 276 — The edit hook fails closed when it times out or crashes

## What

Install the editor hook so that a timeout or crash of the hook refuses the edit where the agent host offers that, and correct the timeout the harness spec assumes for the host.

## Why

The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document. It reports that the Claude Code documentation now has a per-hook setting that blocks the tool call when the hook fails, and a default hook timeout ten times the one `SPEC-24` states. Today a hook the host kills lets the edit through, which is why `MILE-232` had to make the timeout a floor.

## Done means

- A test on the Claude Code adapter shows that a hook that times out or exits abnormally refuses the edit when the fail-closed setting is installed; if the host does not behave as documented, this milestone records that and stays open.
- `urzua harness install` writes the setting for the gate guardrails, and `urzua doctor` reports an installed entry without it.
- `SPEC-24` states the host's real default timeout, with the version it was checked against.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the harness research of 2026-11-16, which the maintainer asked for that day. | **structural** |
