---
Stable-Id: 01M7GY8H1JK3MPJPCC2GKSNRYZ
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 281 — Chat acceptance refuses when something other than a person can answer

## What

Before the chat acceptance of `ADR-102` is offered in a session, detect the things that can answer a question or a permission prompt in a person's place, and refuse the mode when any is present.

## Why

The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document. It found that on Claude Code another hook, a host program or an idle timeout can supply the answer to a structured question, a hook can answer a permission prompt, and free-text answers exist beside the offered choices. An approval must come from none of these.

## Done means

- With a hook configured that can answer questions or permission prompts, the session-start check reports it and chat acceptance is refused, shown by a test.
- An answer given as free text, by timeout or by dismissal writes no approval, shown by a test per case.
- The permission mode of the session is read, and a mode in which a prompt is not guaranteed to reach a person refuses.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the harness research of 2026-11-16, which the maintainer asked for that day. | **structural** |
