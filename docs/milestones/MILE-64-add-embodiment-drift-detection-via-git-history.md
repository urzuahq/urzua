---
Status: Done
Stable-Id: 01M1YNNDMH0SEJNPEV3Y9ZP1QK
Phase: '0'
Track: embodiment-model
Implements: ADR-32
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/fix_drift_git_integration.rs
---
# 64 — Add Embodiment drift detection via git history

## What

`embodiment.consistency` now detects drift: if a `Realized-by` locator changed, per git history, since the `Realized-by` line was last touched, the expected `Embodiment` is `Drift detected` regardless of tier, and a stated value that disagrees is a finding. No stored hash -- the comparison reads git blame/log directly.

## Why

A back-pointer that's merely present proves someone typed a string once, not that the code still matches. Drift detection is what makes `Realized-by` a claim that can go stale and get caught, not a write-once assertion nobody re-checks.

## Done means

`embodiment.consistency` computes `Drift detected` as the expected `Embodiment` when a `Realized-by` locator changed in git history since the `Realized-by` line was last touched, whatever the tier, and reports a stated value that disagrees; no hash is stored, the comparison reads git blame and log, and `fix_drift_git_integration.rs` plants both a drifted and an undrifted record.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
