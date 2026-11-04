---
Status: Done
Stable-Id: 01M1YNNBHC3AAT0QEJXEZR3W2F
Phase: '0'
Track: header-format
Implements: ADR-27
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs, test:rust/crates/urzua-cli/tests/new_integration.rs
---
# 60 — Build urzua new

## What

`urzua new <type> [title]`: fills the type's checked-in template (or synthesizes YAML frontmatter if the type declares that shape with no template) with a fresh stable ID, resolved author, and today's date. Never asks for a number.

## Why

Every other command validates or reports on records that already exist; nothing created one. A tool whose whole point is a well-formed record shouldn't leave record creation to a human copy-pasting the last one.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-177`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
