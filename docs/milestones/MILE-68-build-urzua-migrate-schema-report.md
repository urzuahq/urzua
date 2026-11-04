---
Status: Done
Stable-Id: 01M1YNNFS3073SZAAP376CD603
Phase: '0'
Track: schema-governance
Implements: ADR-22
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/migrate.rs, code:rust/crates/urzua-core/src/migrate.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 68 — Build urzua migrate schema --report

## What

`urzua migrate schema --report --field <Name>`: a read-only preview listing every record that would newly fail if `Name` were added to `required_fields` today, before it's ever added to config.

## Why

Adding a required field to a live corpus is a real risk of silently breaking every record that doesn't have it yet -- this answers "what would break" before the config change that would cause it.

## Done means

`urzua migrate schema --report --field <Name>` lists, read-only, every record that would newly fail if `Name` were added to `required_fields` today, and writes nothing and changes no config; covered in `check_integration.rs` (`SPEC-14`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
