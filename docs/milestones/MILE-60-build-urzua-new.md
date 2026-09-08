# 60 — Build urzua new

> Status: Done
> Stable-Id: 01M1YNNBHC3AAT0QEJXEZR3W2F
> Phase: 0
> Track: header-format
> Implements: ADR-27
> Blocked-on: —

## What

`urzua new <type> [title]`: fills the type's checked-in template (or synthesizes YAML frontmatter if the type declares that shape with no template) with a fresh stable ID, resolved author, and today's date. Never asks for a number.

## Why

Every other command validates or reports on records that already exist; nothing created one. A tool whose whole point is a well-formed record shouldn't leave record creation to a human copy-pasting the last one.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
