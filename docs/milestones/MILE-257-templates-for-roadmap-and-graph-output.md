---
Stable-Id: 01M7FW2E6EC2MS1RZ8XN9SYK0R
Status: Planned
Phase: '2'
Track: discoverability
Blocked-on: null
---
# 257 — Templates for roadmap and graph output

## What

Let `roadmap` and `graph` write a formatted file the way `index` does, from a declared format.

## Why

the audit of 2026-11-15's extensibility experiments found both commands return JSON only, while `index` already has a declared format; a team that wants a roadmap page writes its own script.

## Done means

- A declared format produces a roadmap file and a graph file with `--apply`, and `check` reports when either is out of date.
- This repository's roadmap page is generated that way.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
