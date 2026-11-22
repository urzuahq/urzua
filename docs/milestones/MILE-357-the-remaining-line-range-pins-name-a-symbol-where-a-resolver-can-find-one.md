---
Stable-Id: 01M82K04HN351VS72Y5ZWWNKXA
Status: Planned
Phase: '1'
Track: embodiment-model
Implements: ADR-74
Blocked-on:
---
# 357 — The remaining line-range pins name a symbol where a resolver can find one

## What

The line-range pins left in `docs/` after `MILE-352` name the item they cite by symbol, or are recorded as ranges on purpose because no resolver can name it.

## Why

`MILE-352` retired the ranges that existed because a field and a method share a name, and found four more that exist for other reasons. Three pin a test function the `rust` resolver cannot end: `MILE-191` (`record_edit_hook_integration.rs#L457-L464`) and `MILE-198` (`harness_integration.rs#L363-L381`, `#L383-L417`) cite test functions whose bodies hold a `{` inside a string literal, so `brace_balanced` never closes and `path#name` reports "the end of the item is not found" (the blindness `SPEC-3` states for that end kind). One, `MILE-284` (`CONTRIBUTING.md#L31-L56`), cites a Markdown section, and this repository declares no resolver for `*.md`, although the `heading` end kind exists. Each goes stale whenever lines above it move. Fixing the first needs a decision: a new end kind that skips string literals (end kinds are append-only, `ADR-74` point 5) or a test written without the stray brace. The second is a `locator_resolvers` entry in `.urzua/config.yaml`, a protected path.

## Done means

- `MILE-191` and `MILE-198` pin the test functions they cite by symbol, or this record says, for each range kept, why no symbol can name it.
- `MILE-284` pins the `CONTRIBUTING.md` section by a heading symbol through a declared Markdown resolver, or this record says why the range is kept.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-22 | Filed as planned work while delivering `MILE-352`: nothing built, no evidence yet. | **structural** |
