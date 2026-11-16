---
Stable-Id: 01M7GY86HQ13K6F1MDE68RVDWG
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 279 — The gate in CI runs from the base revision, not the pull request's own build

## What

Run the acceptance gate in CI with a binary and a gate definition the pull request cannot change, and add a host rule restricting who can change the workflow files.

## Why

The adversarial review of 2026-11-15 noted that this repository's gate job runs a binary built from the pull request head, and that a pull request can drop the gate job; `ADR-89` lists both as accepted risks. The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document and found host rules that close them.

## Done means

- The gate job for a pull request uses a released binary or one built from the base revision, shown by a pull request that breaks the engine and still fails the gate.
- A host rule restricts changes to the workflow and action paths, and the CI reference page tells an adopter how to set the same rule.
- `ADR-89`'s accepted risks are amended to say which of the two is now closed.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the harness research of 2026-11-16, which the maintainer asked for that day. | **structural** |
