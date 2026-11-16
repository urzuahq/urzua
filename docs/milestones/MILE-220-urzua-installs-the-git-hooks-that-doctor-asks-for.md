---
Stable-Id: 01M7FVRQKX17381R1VXZWAKYJ7
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 220 — urzua installs the git hooks that doctor asks for

## What

Add a command that wires the pre-commit and pre-push hooks `urzua doctor` reports as missing, and ship a hook definition for the pre-commit framework.

## Why

the audit of 2026-11-15 found that `doctor` warns when git hooks are missing but no command installs them; this repository wires its own with a Makefile target an adopter does not have.

## Done means

- One command installs the pre-commit and pre-push hooks in a repository that has none, and `urzua doctor` then reports the hook checks as passing.
- The command refuses to overwrite an existing hook or a `core.hooksPath` it did not set, and says how to call urzua from it.
- A repository using the pre-commit framework can add urzua with a documented stanza, checked by a test that runs it.
- The session-start warning about missing hooks names the command.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
