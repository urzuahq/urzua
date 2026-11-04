---
Status: Done
Stable-Id: 01M1YNNEQPK3PG0PZPG3K08QFG
Phase: '0'
Track: accountability-identity
Implements: ADR-31
Blocked-on: —
Realized-by: code:rust/crates/urzua-io/src/lib.rs, test:rust/crates/urzua-io/src/lib.rs
---
# 66 — Trust a verified gh login over --by for identity resolution

## What

Identity resolution for `fix --apply` and `new` now checks a verified `gh api user` login first, then `git config user.name`, and only falls back to an explicit `--by` value when neither verified source is available -- reversing the previous priority, where a typed `--by` could silently override an authenticated session.

## Why

An unverified, typed name overriding a verified login is exactly backwards for a field whose entire purpose is accountability -- the strongest available identity signal should always win over the weakest.

## Done means

Identity resolution for `new` and `fix --apply` takes a verified `gh api user` login first, then `git config user.name`, and uses an explicit `--by` only when neither is available; a typed `--by` never overrides a verified login, and unit tests in `urzua-io` cover each order (`resolve_identity_lets_gh_win_over_a_differing_explicit_by` and its siblings).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | `Realized-by` test locator corrected to the unit tests in `urzua-io/src/lib.rs` (`resolve_identity_lets_gh_win_over_a_differing_explicit_by`, `resolve_identity_never_lets_explicit_by_override_a_verified_gh_login`, `resolve_identity_prefers_explicit_by_over_git_config`, `resolve_identity_falls_through_to_git_config_when_gh_is_unavailable`); the old locator `new_integration.rs` only passes `--by`. The resolution order described in the What was later superseded: the 2026-09-10 amendment of `ADR-31` split the ambient tiers so that `--by` now outranks `git config`, the order being a verified `gh` login, then `--by`, then `git config user.name`. `Status` stays Done for what shipped on 2026-09-06. **Why:** the closure audit of #234 found the What stale and the test locator weak. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
