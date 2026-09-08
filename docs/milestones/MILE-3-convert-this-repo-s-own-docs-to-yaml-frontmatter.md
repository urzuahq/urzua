# 3 — Convert this repo's own docs to yaml-frontmatter

> Status: Planned
> Stable-Id: 01M1Y5JDMZ6T2KYMZZC2HKP78T
> Phase: 0
> Track: header-format
> Implements: ADR-33
> Blocked-on: —

## What

A one-time, unshipped conversion pass over this repo's own ~35 ADR/RFC/Spec/Bug/Milestone records, byte-preserving everything below the header.

## Why

The actual dogfooding step ADR-33 named as the real forcing function for this work -- no adopter is under pressure to migrate, so this repo doing it first is the only evidence the mechanism is safe.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Cleared `Blocked-on` (was `BUG-3`, `Status: Fixed` for a while, caught by `blocked-on.stale` -- MILE-83 -- immediately after `Blocked-on` moved from prose into a checked header field). **Why:** the blocker had silently resolved and nobody had revisited this milestone since; found live during backlog triage, confirmed by the new rule firing on this exact case before this fix. | **substantive** |
