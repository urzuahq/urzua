---
Status: Done
Stable-Id: 01M1YNNAAMTDJMEHET3W5XK1QE
Phase: '0'
Track: roadmap-tracking
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/doctor.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 58 — Build urzua doctor

## What

`urzua doctor`: reports on the tool's own configuration and invocation health (does a config exist, does it parse, is `check` actually wired into CI) rather than on record content.

## Why

A checker that's correct but never invoked is indistinguishable from no checker at all -- doctor answers "is this actually running," a different question than "did this find anything."

## Done means

`urzua doctor` reports on the tool's own health rather than on record content: whether a config exists, whether it parses, whether a record type declares what the rules need (such as required fields), and whether `check` is wired into CI and the local git hooks; it exits non-zero on an unhealthy setup, each check is covered in the integration tests, and it runs clean on urzua's own repository.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
