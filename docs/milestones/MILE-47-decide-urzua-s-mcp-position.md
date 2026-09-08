# 47 — Decide Urzua's MCP position

> Status: Planned
> Stable-Id: 01M1YN858DBHWE5XJ2JFEVY1F6
> Phase: 2
> Track: escalation
> Blocked-on: —

## What

Write an RFC deciding Urzua's MCP position: whether to ship an MCP server at all, and if so, why read-only (exposing rejected/superseded decisions so an agent doesn't re-propose what was already tried) rather than write-capable.

## Why

MILE-29 already names "a read-only MCP server" as a build target, but no RFC or ADR backs that stance -- unlike every other build-track milestone, its own `Implements` field is empty. The position (why read-only, why this specific data) needs deciding before the build, not assumed by the milestone's own title.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
