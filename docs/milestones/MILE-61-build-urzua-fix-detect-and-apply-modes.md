---
Status: Done
Stable-Id: 01M1YNNC28J6KNP3D6PC6H7SCY
Phase: '0'
Track: embodiment-model
Implements: ADR-15, ADR-19, ADR-20
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/fix.rs, code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs
---
# 61 — Build urzua fix detect and apply modes

## What

`urzua fix`: detect mode reports fields whose stated value disagrees with what's computed from `Realized-by` evidence; `fix --apply --ids <records> --by <you>` writes the computed value back, one field's line only, with a required identity and an appended revision-log entry. Refuses outright on a record with no revision log.

## Why

`check` can tell you a field is wrong; nothing closed the loop by writing the mechanically-correct value back. Apply mode is gated hard (explicit ids or --force, a resolved identity, an auditable revision-log entry) because a bulk unreviewed write is exactly the failure mode a governance tool can't afford.

## Done means

`urzua fix` in detect mode lists fields whose stated value disagrees with the value computed from `Realized-by` evidence; `fix --apply --ids <records> --by <you>` writes the computed value to that one field's line only, with a resolved identity and an appended revision-log row, and refuses outright on a record with no revision log; without explicit ids it refuses unless forced; covered in `fix_drift_integration.rs` (`SPEC-8`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
