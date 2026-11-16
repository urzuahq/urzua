---
Stable-Id: 01M7GY8VHRGKFQPA9XACW1J1P3
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 283 — A change to the hook settings during a session is refused or reported

## What

Use the agent host's configuration-change event, where it exists, to refuse or report an edit to the hook settings made in the middle of a session.

## Why

The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document and reports a hook event for configuration changes. The harness detects a changed translator by its hash, but only when the next hook runs.

## Done means

- A test shows a mid-session change to the installed hook settings is refused or reported at once on the Claude Code adapter, or the milestone records that the host does not behave as documented.
- The session-start check and this event agree on what counts as tampering.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the harness research of 2026-11-16, which the maintainer asked for that day. | **structural** |
