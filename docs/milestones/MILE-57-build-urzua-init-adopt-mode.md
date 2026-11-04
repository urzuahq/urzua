---
Status: Done
Stable-Id: 01M1YNN9K5B2V3D7MSGD9HWNG1
Phase: '0'
Track: roadmap-tracking
Implements: SPEC-5
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs, test:rust/crates/urzua-cli/tests/acceptance.rs
---
# 57 — Build urzua init adopt mode

## What

`urzua init` adopt mode: proposes `.urzua/config.toml` from an existing corpus's real directories and header conventions, never moving or rewriting a single record. `--dry-run` byte-identical to the real run.

## Why

The tool has to be adoptable without a rewrite for "one engine instead of a fork per repo" to mean anything -- adopt mode is the only path that doesn't ask an existing corpus to change shape first.

## Done means

`urzua init` adopt mode proposes a config from an existing corpus's real directories and header conventions without moving or rewriting a record, `--dry-run` output is byte-identical to the real run's, and a corpus outside `docs/` or with a colliding type name is adopted or refused with a message rather than silently dropped; the written config is `.urzua/config.yaml`, which supersedes the `.toml` named in the What.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
