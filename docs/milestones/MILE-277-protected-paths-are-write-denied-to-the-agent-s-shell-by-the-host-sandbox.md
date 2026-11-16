---
Stable-Id: 01M7GY7VWZHGXSS04VS11YGWK1
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 277 — Protected paths are write-denied to the agent's shell by the host sandbox

## What

Have `urzua harness install` write the agent host's sandbox and deny rules so that a shell command cannot write a protected path or the harness's own state, and have `doctor` check they are in force.

## Why

The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document. Every vendor says command parsing is not a security boundary; the strongest agents deny writes at the operating-system level. The urzua harness only reports a shell write after it has happened (`ADR-89`, accepted risks), which is the gap the approvals of `ADR-102` would inherit.

## Done means

- A test shows that with the installed rules a shell redirect, `sed -i`, `git apply` and a script that opens the file itself each fail to write a protected path on Linux, or the milestone records which route the sandbox does not cover.
- The install writes deny rules for the paths the `authority` table protects and for the harness state under `.urzua`, and removes them on uninstall.
- `urzua doctor` reports a repository whose protected paths the agent's shell can still write, naming the setting to add.
- The agent harness page states what the sandbox covers and does not (the edit tools, MCP servers and hooks run outside it).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the harness research of 2026-11-16, which the maintainer asked for that day. | **structural** |
