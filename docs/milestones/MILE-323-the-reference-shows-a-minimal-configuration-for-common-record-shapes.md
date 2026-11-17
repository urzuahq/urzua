---
Stable-Id: 01M7KB9MZA8EQ92SHZPN6QJP2E
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 323 — The reference shows a minimal configuration for common record shapes

## What

Add to the reference a short, complete configuration example for each of a few common shapes of record type, and say in the `init` output where they are.

## Why

`urzua init --types` offers four profiles. A repository that wants a type with a checklist, a type that points at another, or a type with gated statuses has one worked example to copy: this repository's own configuration of about 700 lines, most of it specific to how this repository works. The agent that reported this on 2026-11-16 named it as the largest time cost of the bootstrap. The engine has no opinion on which types a repository declares, so the examples are organised by mechanism, not by a workflow's vocabulary.

## Done means

- The reference has a complete, loadable example of under forty lines for each of: a type with a checklist whose state is computed, a type whose field points at another type, a type with a gated status, and a type with a declared relation pair; each is loaded by a test so it cannot go stale.
- `urzua init`'s report names the page.
- The type-directory statement in the commands reference is corrected under `MILE-270`, which already tracks it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-16 by an agent setting up a new adopting repository on release 0.4.6, relayed at the maintainer's request; checked against the code the same day. | **structural** |
