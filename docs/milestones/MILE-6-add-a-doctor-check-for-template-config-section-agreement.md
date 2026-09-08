---
Status: Planned
Stable-Id: 01M1Y5JF83Z09VVDSV35G4BS2W
Phase: '0'
Track: section-checks
Implements: RFC-17, RFC-18
Blocked-on: 'Milestone: Build the closed section-parser and required_sections config schema'
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
