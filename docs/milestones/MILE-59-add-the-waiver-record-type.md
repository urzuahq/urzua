---
Status: Done
Stable-Id: 01M1YNNAYPRGKFY23FMFWCQ907
Phase: '0'
Track: accountability-identity
Implements: ADR-11
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/waiver.rs, test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs
---
# 59 — Add the waiver record type

## What

The `waiver` record type: a reviewed exception to a rule is its own record (`Rule`, `Scope`, `Reason`, optional `Expires`), never a config-level ignore list. A waived finding stays listed, just excluded from the blocking exit code.

## Why

An ignore list is unreviewable and grows forever; a waiver record is a normal record with an author, a reason, and an expiry -- reviewable in the same diff as everything else.

## Done means

`waiver` is a configured record type (`Rule`, `Scope`, `Reason` required, optional `Expires`) with a template, so `urzua new waiver` works end to end; a finding matched by a waiver stays listed but is excluded from the blocking exit code, an expired or unmatched waiver is reported, and `docs/waiver/` holds this repository's own reviewed exceptions (`SPEC-10`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-04 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-09-07 | Reopened: `Done` was inaccurate. `waiver.rs`'s parsing/matching logic existed and was tested, but the record type was never actually registered in `.urzua/config.toml`'s `record_types`, so `urzua new waiver` failed outright and no corpus author could ever create one -- ADR-11's own stated consequence ("must be added to `.urzua/config.toml`'s `record_types`... no special-casing") was never carried out. **Why:** found live, reasoning about why ADR-1's stale `Embodiment` couldn't be waived -- there was no way to create a waiver record at all. | **substantive** |
> | 2026-09-07 | Fixed: added `[record_types.waiver]` (`dir = "docs/waiver"`, `required_fields = ["Rule", "Scope", "Reason"]`) and `.urzua/templates/waiver.md`. Verified `urzua new waiver` end-to-end against an isolated scratch corpus (not this one, since there is no real waiver to add yet). `docs/waiver/` itself is left uncreated, same as every other type's directory before its first real record -- no placeholder file needed. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
