---
Status: Done
Stable-Id: 01M1YNNCJQKWVEVQWPQ3ZCHJMV
Phase: '0'
Track: governance-process
Implements: ADR-30
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/audit.rs, test:rust/crates/urzua-cli/tests/acceptance.rs
---
# 62 — Build urzua audit

## What

`urzua audit`: cross-record reconciliation -- supersession reciprocity and dangling cross-references -- reusing `check`'s own rule functions rather than a second implementation. Read-only; never writes.

## Why

A bulk cross-reference rewrite is a real data-loss risk without a review step, so `audit` reports what's wrong across records without ever attempting to fix it itself.

## Done means

`urzua audit` reports supersession reciprocity and dangling cross-references across records by calling `check`'s own rule functions rather than a second implementation, is read-only and writes nothing, and the acceptance suite runs it (`SPEC-11`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
