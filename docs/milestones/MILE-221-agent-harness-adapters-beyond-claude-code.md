---
Stable-Id: 01M7FVRYMJCKFGK5JNSST33V1M
Status: Planned
Phase: '2'
Track: integrations
Blocked-on: null
---
# 221 — Agent harness adapters beyond Claude Code

## What

Ship harness adapters for at least two more coding agents under the adapter contract of `SPEC-24`, with the same guardrails where the agent's hook surface allows.

## Why

the audit of 2026-11-15 found one adapter. The mission in `ADR-101` speaks of agents in general, and a team using another agent gets only the MCP server and CI.

## Done means

- `urzua harness adapters` lists at least three adapters.
- Each new adapter installs with `urzua harness install`, and `urzua doctor` reports which guardrails it provides and which the agent cannot support.
- An acceptance move by the agent is refused or reported under each new adapter, shown by a test per adapter.
- The agent harness reference page documents each adapter.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Decided: Codex and Cursor are the next two adapters. **Why:** the maintainer chose them on 2026-11-16 when asked which harnesses come after Claude Code. | **substantive** |
