---
Stable-Id: 01M7FVTAF2GWVRZN3DR7T036SB
Status: Planned
Phase: '2'
Track: integrations
Blocked-on: null
---
# 224 — The MCP server exposes the roadmap

## What

Add a read-only MCP tool that returns the `roadmap` report, so an agent can ask what is planned, blocked and next.

## Why

the audit of 2026-11-15 found five MCP tools and none for the roadmap, although `urzua roadmap` already returns the data.

## Done means

- The MCP tool list includes a roadmap tool, and its result equals `urzua roadmap` for the same repository, shown by a test.
- The MCP reference page documents the tool.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
