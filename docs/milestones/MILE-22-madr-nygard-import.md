---
Status: Planned
Stable-Id: 01M1Y5JQRE969D564R9FMN4GDW
Phase: '1'
Track: interop
Implements: —
Blocked-on: —
---
# 22 — MADR/Nygard import

## What

A one-way, non-destructive import path from the MADR/Nygard ADR convention.

## Why

The real, most common adoption shape in the wild for ADR-only corpora; not yet scoped.

## Done means

`urzua import` reads a MADR or Nygard ADR directory into the configured ADR type without modifying the source, maps what it can, reports what it cannot map, and a second run is a no-op; a fixture corpus in each convention imports with zero unwaived findings from `check`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |