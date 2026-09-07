# 0006 — Add a doctor check for template/config section agreement

> Status: Planned
> Stable-Id: 01M1Y5JF83Z09VVDSV35G4BS2W
> Phase: 0
> Track: section-checks
> Implements: RFC-0017

## What

doctor verifies each type's template contains a heading for every section its config declares required.

## Why

Nothing today checks that a template and its config haven't drifted apart -- the same class of silent gap the header_shape/template bug already demonstrated once.

## Blocked on

Milestone: Build the closed section-parser and required_sections config schema

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
