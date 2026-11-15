---
Stable-Id: 01M7FW2ZC12RT8G8WBB81R90JJ
Status: Planned
Phase: '2'
Track: roadmap-tracking
Blocked-on: null
---
# 259 — Relations can constrain the target's type, and graph shows relation pairs

## What

Let a relation declare which record types it may point at, and include relation-pair edges in `graph`.

## Why

the audit of 2026-11-15's extensibility experiments found a story parented directly to a product requirements record passing, and `graph` returning nothing for a corpus linked only by relation pairs.

## Done means

- A relation that declares its target types reports a link to another type.
- `urzua graph` returns the edges of every declared relation pair.
- A configuration that declares no target types behaves as before.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
