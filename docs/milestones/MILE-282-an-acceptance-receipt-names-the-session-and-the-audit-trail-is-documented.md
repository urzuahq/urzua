---
Stable-Id: 01M7GY8PECVEGPZAFKTFXVXR3R
Status: Planned
Phase: '2'
Track: accountability-identity
Blocked-on: null
---
# 282 — An acceptance receipt names the session, and the audit trail is documented

## What

Put the session identifier, the agent identifier when a subagent made the call, and a reference to the transcript into each acceptance receipt, and document how to export the agent host's own audit log.

## Why

The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document. Hook input identifies the session and the subagent but not the person, and the hosts export tool calls, permission waits and hook runs as telemetry.

## Done means

- A receipt written by the harness carries the session identifier and, for a subagent call, the agent identifier.
- The agent harness page shows how to turn on the host's telemetry and which events record a decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the harness research of 2026-11-16, which the maintainer asked for that day. | **structural** |
