---
Status: Planned
Stable-Id: 01M1Y5JEPZRBT40QA2DEXT2CM2
Phase: '0'
Track: section-checks
Implements: RFC-17
Blocked-on: MILE-4, MILE-98
---
# 5 — Build the y-statement content shape

## What

Extract a Decision section's TL;DR block and check it contains the five Y-statement structural phrases, word-boundary matched, with a distinct error when the marker exists but no extractable block follows.

## Why

The first real content shape, proving the section-parser design; validated against two independent real implementations that converged on the same extraction/matching approach.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Reshaped by `ADR-53`. **Why:** filed as "build the y-statement content shape" when a shape meant a Rust function. `RFC-33` makes a shape a parameter to `pattern`, so the work is a declared shape plus whatever `pattern` needs to support it -- not a rule. Blocked on the document model, which is what locates a Decision section's TL;DR block in the first place. | **substantive** |
> | 2026-09-17 | `Blocked-on` now names the blocking record rather than describing it. **Why:** it held a sentence (*'Milestone: Build the closed section-parser...'*) which is legal -- `Blocked-on` is a declared `narrative_field`, and prose is what those are for, since "blocked on a decision about X" is a real value no pointer expresses. Changed because in *these three* cases the prose described exactly one existing record, so naming it lets `narrative-field.stale` see the dependency and report when the target goes terminal. Not a defect being fixed; a narrative value replaced by a more useful one. | **structural** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
