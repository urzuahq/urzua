---
Status: Planned
Stable-Id: 01M1YN82MMNP7V6P4WB6FK7XKB
Phase: '1'
Track: discoverability
Blocked-on: —
---
# 42 — Auto-generated INDEX.json for fast corpus discovery

## What

A generated `INDEX.json` (or similar) summarizing the corpus -- record type, status, title -- so an agent can query a compact index instead of reading every record.

## Why

`explain`/`graph` already answer "which records govern this file" and "what's the relationship graph," but nothing answers "what exists in this corpus at all" without reading every file. An agent with a large corpus and a token budget needs the cheap, coarse answer before it needs the precise one.

## Forward note: resolving the index's location safely

A cached, disk-persisted artifact like `INDEX.json` is only as correct as the working-copy root it
was resolved against. Resolving that root from an ambient environment variable, or from the current
process's own working directory, is a real, previously-observed way for a multi-worktree or
parallel-branch setup to silently read a stale cache built for a *different* checkout than the one
actually being operated on — neither of those two sources is guaranteed to match the file actually
being read or written.

The robust pattern: derive the root explicitly from the path of the file actually being operated on
(e.g. `git rev-parse --show-toplevel` run with its directory as `cwd`), never from an environment
variable or the calling process's own `cwd`. Whoever builds this milestone should resolve
`INDEX.json`'s location the same way — from the path being checked, not an ambient default — or
invalidate/recompute it per-checkout unambiguously.

Nothing in `urzua-core` has this exposure today (every rule recomputes fresh from the records it's
handed each invocation); this is purely a forward note for a command that doesn't exist yet.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-09 | Added a forward note on safely resolving `INDEX.json`'s location relative to the actual working-copy root, not an ambient environment variable or process `cwd`. **Why:** a generalized, previously-observed stale-cache/worktree-resolution bug pattern, surfaced while planning MILE-90 — recorded here since this is the durable home for whoever eventually builds this milestone, not the ephemeral session plan that surfaced it. | **structural** |
