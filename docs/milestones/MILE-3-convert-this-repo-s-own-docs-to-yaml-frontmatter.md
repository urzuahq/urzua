---
Status: Done
Stable-Id: 01M1Y5JDMZ6T2KYMZZC2HKP78T
Phase: '0'
Track: header-format
Implements: ADR-33
Blocked-on: —
---
# 3 — Convert this repo's own docs to yaml-frontmatter

## What

A one-time, unshipped conversion pass over this repo's own 166 ADR/RFC/Spec/Milestone/Bug records
(all six configured types per ADR-33's amendment; `waiver` has none yet, so only its config
declaration changed), byte-preserving everything below the header.

## Why

The actual dogfooding step ADR-33 named as the real forcing function for this work -- no adopter is under pressure to migrate, so this repo doing it first is the only evidence the mechanism is safe.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Cleared `Blocked-on` (was `BUG-3`, `Status: Fixed` for a while, caught by `blocked-on.stale` -- MILE-83 -- immediately after `Blocked-on` moved from prose into a checked header field). **Why:** the blocker had silently resolved and nobody had revisited this milestone since; found live during backlog triage, confirmed by the new rule firing on this exact case before this fix. | **substantive** |
> | 2026-09-08 | `Status: Done`; scope corrected from "~35 ADR/RFC/Spec/Bug/Milestone" to the actual 166 files across all six configured types (ADR-33's amendment). Migrated via a scratch, uncommitted Rust tool reusing `header::parse_with_shape` -- not shipped, per ADR-33's explicit "not a new public subcommand" requirement. Verified by a real before/after field-set-equality comparison against every file (zero mismatches), `check docs/` (zero blocking findings, `header.deprecated-shape`/`type.no-declared-spec` both silent), `graph` (zero dangling edges), and a dashboard regeneration confirming real field values render instead of `—`. | **substantive** |
