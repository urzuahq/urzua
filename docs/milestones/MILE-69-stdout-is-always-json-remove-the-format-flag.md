---
Status: Done
Stable-Id: 01M1YNNG9RQEZYDDHBQDN5AJDW
Phase: '0'
Track: governance-process
Implements: ADR-23
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 69 — Stdout is always JSON; remove the --format flag

## What

Removed `--format`: `check`, `fix`, `explain`, `graph`, and `migrate schema --report` now emit exactly one JSON object on stdout, unconditionally, with no human-readable rendering anywhere.

## Why

Two output shapes mean two things that can silently drift apart, and an agent piping stdout, a script, and a person reading a terminal should all see exactly the same thing -- one shape is a stronger guarantee than a flag that happens to default correctly.

## Done means

`check`, `fix`, `explain`, `graph` and `migrate schema --report` each emit exactly one JSON object on stdout, unconditionally, with no `--format` flag and no human-readable rendering; a test asserts the flag is rejected and stdout parses as one JSON value.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
