---
Stable-Id: 01M7GY81CTD2A94A06SRNN9ENM
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 278 — The post-command hook is silent unless a command changed a governed file

## What

Make the post-command hook report only what the command itself changed, say nothing when it changed nothing governed, and send each message to one audience.

## Why

The maintainer asked on 2026-11-16 why the same multi-line report appears after every command. The hook compares the working tree with a saved baseline, so after a pull it repeats the same protected-path changes on every later command, including read-only ones, and it writes the same text to both the person and the agent. The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document and reports that the host can tell a hook which files a shell command changed.

## Done means

- A read-only command produces no hook output, shown by a test.
- A change brought in by `git pull`, `git checkout` or a merge is not reported as the agent's edit, and is not repeated after later commands.
- A gated move or a protected-path change made by the command is reported once: one line to the agent, and a message to the person only for those two cases and for a hook that could not run.
- The hook does not start the engine for a command the host reports as having changed no file, where the host provides that.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** The maintainer asked about the repeated post-command report on 2026-11-16; the fix follows the harness research of that day. | **structural** |
