---
Status: Done
Stable-Id: 01M1YNNF8F2GD7BAQVX9E5Q7K5
Phase: '0'
Track: roadmap-tracking
Implements: ADR-3, ADR-21
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/migrate.rs, code:rust/crates/urzua-core/src/migrate.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 67 — Build urzua migrate ids

## What

`urzua migrate ids`: backfills a collision-free `Stable-Id` (ULID) into every record lacking one, without touching the filename or any cross-reference -- the display number stays exactly what it already is. Dry-run by default.

## Why

ADR-3's stable/display split only matters if every record actually has a stable ID -- this is what makes the split real for a corpus that predates it, without forcing a disruptive renumbering to get there.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
