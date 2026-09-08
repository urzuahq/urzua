---
Status: Planned
Stable-Id: 01M1Y5JE63FXMT7PV7QJQN7BXV
Phase: '0'
Track: section-checks
Implements: RFC-17
Blocked-on: —
---
# 4 — Build the closed section-parser and required_sections config schema

## What

A shared, closed function locating every ## section's body, mirroring header.rs's closed-header model, plus a required_sections config schema with a composable content shape per section.

## Why

check validates header fields today but nothing validates what's inside a required section's prose -- SPEC-2 names this rule category and it has never been built.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
