---
Stable-Id: 01M7GX85JSRQ2MWPC8A20JD9GN
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 273 — Build acceptance in chat: the harness asks the person, records the answer and allows that one edit

## What

Build the opt-in mode of the `acceptance` guardrail that `ADR-102` proposes: the refusal tells the agent to ask the person a structured question, a hook turns the person's answer into a one-use approval the agent cannot write, the matching edit is allowed, and the harness writes the acceptance receipt.

## Why

The maintainer reported on 2026-11-16 that every acceptance ends with the agent saying it has no permission and sending them to edit the file by hand, and asked to approve from a question in the chat. Blocked until `ADR-102` is accepted.

## Done means

- `ADR-102` is accepted, and `ADR-89` carries an amendment stating what an approval given in chat proves and does not prove.
- On the Claude Code adapter a recorded test shows what a hook receives as the person's answer to a structured question, whether the agent can supply that answer itself, and whether a hook's confirmation prompt is shown in each permission mode; the adapter offers only what those results support.
- Where the host supports a tool that requires the person's interaction on every call, an urzua tool asks for the acceptance with a prompt urzua wrote, and on approval makes the status change and writes the receipt itself (`ADR-102`, amendment of 2026-11-16); the chat question is offered only where the host has no such tool.
- With the mode on, an agent's move to a reserved status with no approval is refused with the instruction to ask, and after the person chooses accept the same edit is allowed once, shown by a test.
- An approval is refused for another record, another status, a changed body, another session or after its expiry, shown by a test per case.
- The agent cannot write an approval: an edit or shell write to where approvals are kept is refused or reported like any protected harness state.
- The harness writes the acceptance fields and a revision-log row naming the method, and a rule reports a record at a reserved status with no receipt.
- A configuration that does not declare the mode behaves as before, shown by a test.
- The installed instructions block tells the agent to ask the person, and this repository's own configuration uses the mode.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** The maintainer asked for acceptance by a question in the chat on 2026-11-16; `ADR-102` is drafted as Proposed and this tracks the build. | **structural** |
> | 2026-11-16 | Added a Done-means item for the approval tool as the first mechanism. **Why:** `ADR-102`'s amendment of 2026-11-16, accepted by the maintainer, makes it the primary route and the chat question the fallback. | **substantive** |
