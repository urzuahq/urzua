---
Stable-Id: 01M6XEAT4ZYYHJ31X2XYTGB59D
Status: Open
---
# COND-0006 — The host ruleset on main requires the ci check

## Signal

Met when a repository ruleset on `urzuahq/urzua` targets `main` only (not the `release` branch and not `v*` tags), requires a pull request with 0 approvals, requires the `ci` status check from GitHub Actions, blocks force pushes and deletions, and has no bypass actor, as `ADR-95` section 4 states. The maintainer reports it in `Met-by`, after reading the ruleset in the repository settings; the steps are in `docs/reference/release-app-setup.md`. A ruleset that is only drafted, targets other refs, or lists a bypass actor is not met.

## Mitigation of ADR-89 risk 13

The ruleset should also require branches to be up to date before merging (the "Require branches to be up to date before merging" option of the required status check), or the repository should use a merge queue. That is the mitigation `ADR-89` names for accepted risk 13 (a stale base with no `authority`, where the head decider table is judged); it is not part of the Signal above, which stays as written.

## Why this is a condition

`ADR-95` states the intent and says the ruleset "is required and not yet applied": it is a repository setting, not a file, so no change in this repository can make it true and `urzua check` cannot read it. Without it nothing gates a merge to `main`, so the CI evidence and the release pull request check are advisory. `MILE-216` (a distinct agent identity with `CODEOWNERS` and branch protection) builds on the same host setting and waits on it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-08 | Initial record. **Why:** `ADR-95` leaves the `main` ruleset requiring `ci` pending as a maintainer-only host setting, and no record tracked it. | **structural** |
> | 2026-11-09 | Added a note that the ruleset should also require branches to be up to date before merging, or use a merge queue, as the mitigation for `ADR-89` accepted risk 13. The Signal is unchanged. **Why:** maintainer decision of 2026-11-09 accepting that risk on the condition that this mitigation is available. | **substantive** |
