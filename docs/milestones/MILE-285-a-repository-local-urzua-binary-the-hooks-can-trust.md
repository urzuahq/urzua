---
Stable-Id: 01M7H3V5X1MEQ2W1NBZNS402C1
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 285 — A repository-local urzua binary the hooks can trust

## What

The hooks can run a urzua binary installed inside the repository, so an adopter does not depend on `PATH` for the judge, and nothing a command run during the session writes can change which binary the hooks run.

## Why

An adopter wants the hooks to use a pinned binary installed in the repository without relying on `PATH`. A first attempt pinned the binary's hash in the harness lock, and a review on 2026-11-16 showed that a shell command that plants a binary and rewrites the pin replaces the judge with nothing reported: the lock is a file the agent's shell can rewrite, so it cannot be the trust anchor. That attempt was removed from the pull request that carried it; this milestone is the version with a different anchor, the binary's hash embedded in the installed hook command beside the translator's hash, which the agent host loads at session start.

## Done means

- The sha256 of the repository's binary is embedded in the installed hook command beside the translator's hash, so a write during the session cannot change which binary runs.
- A planted or replaced binary is not executed and is reported, including when the lock is rewritten in the same command; a test plants a script, rewrites the lock and runs the hooks, and asserts the script never ran.
- A binary that git tracks is refused, with a message.
- `urzua harness status`, `urzua doctor` and the session-start check report a binary whose hash differs from the one the hook command embeds.
- A repository that installs no local binary behaves exactly as before: `URZUA_BIN`, then `PATH`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
