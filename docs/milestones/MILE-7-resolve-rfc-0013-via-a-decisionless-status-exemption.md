---
Status: Planned
Stable-Id: 01M1Y5JFS4DE9EE9HFVD4H4QHP
Phase: '0'
Track: section-checks
Implements: RFC-13
Blocked-on: MILE-5, RFC-33
---
# 7 — Resolve RFC-13 via a decisionless-status exemption

## What

A configured set of statuses (e.g. Withdrawn) exempt from required_sections and shape checks entirely.

## Why

A tombstone record for a vacated number never made a real decision -- requiring a Y-statement from it forces it to invent content that lies to satisfy the checker.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Reshaped by `ADR-53`, and **not** dissolved by it. **Why:** worth stating, because opt-in looks like it answers this and does not. Opt-in turns a rule off for a whole repository; this needs the rule **on** while exempting records in a declared state -- a `Withdrawn` ADR should not be asked for sections a live one must have. That is a rule *option*, not a rule level, so it lands with the function vocabulary rather than before it. | **substantive** |
> | 2026-09-17 | `Blocked-on` now names the blocking record rather than describing it. **Why:** it held a sentence (*'Milestone: Build the closed section-parser...'*) which is legal -- `Blocked-on` is a declared `narrative_field`, and prose is what those are for, since "blocked on a decision about X" is a real value no pointer expresses. Changed because in *these three* cases the prose described exactly one existing record, so naming it lets `narrative-field.stale` see the dependency and report when the target goes terminal. Not a defect being fixed; a narrative value replaced by a more useful one. | **structural** |
