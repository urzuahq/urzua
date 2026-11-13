---
Status: Done
Stable-Id: 01M1YNNEQPK3PG0PZPG3K08QFG
Phase: '0'
Track: accountability-identity
Implements: ADR-31
Blocked-on: —
Realized-by: test:rust/crates/urzua-io/src/lib.rs#resolve_identity_lets_gh_win_over_a_differing_explicit_by@sha256:933601e472d8c8dd0cca0eb280e5fdee6eeab2bf79f327a08d152959f3180bd1, code:rust/crates/urzua-io/src/lib.rs#resolve_identity_with@sha256:5455c706f8b1d8328c4424c0f8ca6bf2c5b65ee55a31633c34759c4f91de9855, test:rust/crates/urzua-io/src/lib.rs#resolve_identity_falls_through_to_git_config_when_gh_is_unavailable@sha256:9ac52b084d7f2844ffb956701e44cba4006d8f4b049adff7b2db81b942345465, test:rust/crates/urzua-io/src/lib.rs#resolve_identity_prefers_explicit_by_over_git_config@sha256:dbb232ebd079632192447331bc0808a997ab4239a34943758192885f214e4cff, test:rust/crates/urzua-io/src/lib.rs#resolve_identity_never_lets_explicit_by_override_a_verified_gh_login@sha256:d6863a7df2bca34c8c250f442f8df5bf7743a5d23457bdf2f95c3d1e4d1ce376
---
# 66 — Trust a verified gh login over --by for identity resolution

## What

Identity resolution for `fix --apply` and `new` now checks a verified `gh api user` login first, then `git config user.name`, and only falls back to an explicit `--by` value when neither verified source is available -- reversing the previous priority, where a typed `--by` could silently override an authenticated session.

## Why

An unverified, typed name overriding a verified login is exactly backwards for a field whose entire purpose is accountability -- the strongest available identity signal should always win over the weakest.

## Done means

- Identity resolution for `new` and `fix --apply` takes a verified `gh api user` login first. — evidence: test:rust/crates/urzua-io/src/lib.rs#resolve_identity_lets_gh_win_over_a_differing_explicit_by@sha256:933601e472d8c8dd0cca0eb280e5fdee6eeab2bf79f327a08d152959f3180bd1, code:rust/crates/urzua-io/src/lib.rs#resolve_identity_with@sha256:5455c706f8b1d8328c4424c0f8ca6bf2c5b65ee55a31633c34759c4f91de9855
- A typed `--by` never overrides a verified login. — evidence: test:rust/crates/urzua-io/src/lib.rs#resolve_identity_lets_gh_win_over_a_differing_explicit_by@sha256:933601e472d8c8dd0cca0eb280e5fdee6eeab2bf79f327a08d152959f3180bd1
- `git config user.name` is used when no verified login is available. — evidence: test:rust/crates/urzua-io/src/lib.rs#resolve_identity_falls_through_to_git_config_when_gh_is_unavailable@sha256:9ac52b084d7f2844ffb956701e44cba4006d8f4b049adff7b2db81b942345465
- An explicit `--by` is used only when neither the verified login nor `git config user.name` is available. — waived: superseded-by-decision (ADR-31) -- the ADR-31 amendment of 2026-09-10 ranks an explicit --by above git config user.name, and resolve_identity_prefers_explicit_by_over_git_config tests that order
- Unit tests in `urzua-io` cover each order. — evidence: test:rust/crates/urzua-io/src/lib.rs#resolve_identity_prefers_explicit_by_over_git_config@sha256:dbb232ebd079632192447331bc0808a997ab4239a34943758192885f214e4cff, test:rust/crates/urzua-io/src/lib.rs#resolve_identity_never_lets_explicit_by_override_a_verified_gh_login@sha256:d6863a7df2bca34c8c250f442f8df5bf7743a5d23457bdf2f95c3d1e4d1ce376

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | `Realized-by` test locator corrected to the unit tests in `urzua-io/src/lib.rs` (`resolve_identity_lets_gh_win_over_a_differing_explicit_by`, `resolve_identity_never_lets_explicit_by_override_a_verified_gh_login`, `resolve_identity_prefers_explicit_by_over_git_config`, `resolve_identity_falls_through_to_git_config_when_gh_is_unavailable`); the old locator `new_integration.rs` only passes `--by`. The resolution order described in the What was later superseded: the 2026-09-10 amendment of `ADR-31` split the ambient tiers so that `--by` now outranks `git config`, the order being a verified `gh` login, then `--by`, then `git config user.name`. `Status` stays Done for what shipped on 2026-09-06. **Why:** the closure audit of #234 found the What stale and the test locator weak. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
