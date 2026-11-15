---
Stable-Id: 01M7FW2P0A00N7N68T7GK1NK65
Status: Planned
Phase: '3'
Track: engine-extensibility
Blocked-on: null
---
# 258 — Lifecycle hooks around commands

## What

Let a configuration name a command to run before or after `new`, `fix --apply` and `waive`, with the report on standard input.

## Why

the audit of 2026-11-15's extensibility experiments found no hook surface apart from the agent harness.

## Done means

- A decision record states which commands have hooks and what a failing hook does.
- A declared hook runs with the report on standard input, and its failure is in the command's report.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
