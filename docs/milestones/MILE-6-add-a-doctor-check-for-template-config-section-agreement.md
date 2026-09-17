---
Status: Planned
Stable-Id: 01M1Y5JF83Z09VVDSV35G4BS2W
Phase: '0'
Track: section-checks
Implements: RFC-17, RFC-18
Blocked-on: MILE-4
---
# 6 — Add a doctor check for template/config section agreement

## What

doctor verifies each type's template contains a heading for every section its config declares required.

## Why

Nothing today checks that a template and its config haven't drifted apart -- the same class of silent gap the header_shape/template bug already demonstrated once.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Added `RFC-18` to `Implements`. **Why:** RFC-18 (widening `doctor` beyond CI-wiring health) explicitly folds this milestone in, and names a real gap this milestone's own text hadn't caught: checking a template *exists* isn't the same as checking it has a heading for every required section — narrower than "template/config section agreement" actually promises. | **substantive** |
> | 2026-09-17 | Blocked-on recorded against `MILE-4`. **Why:** doctor cannot verify a template carries a heading for every declared section until sections are declared, which is the document model's job. | **substantive** |
> | 2026-09-17 | `Blocked-on` now names the blocking record rather than describing it. **Why:** it held a sentence (*'Milestone: Build the closed section-parser...'*) which is legal -- `Blocked-on` is a declared `narrative_field`, and prose is what those are for, since "blocked on a decision about X" is a real value no pointer expresses. Changed because in *these three* cases the prose described exactly one existing record, so naming it lets `narrative-field.stale` see the dependency and report when the target goes terminal. Not a defect being fixed; a narrative value replaced by a more useful one. | **structural** |
