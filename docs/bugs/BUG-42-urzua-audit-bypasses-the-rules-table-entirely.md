---
Stable-Id: 01M2VGDW7MRRMPV29GYHQDPX2M
Status: Fixed
Found-in: 'A code review of the unreleased diff since v0.3.0, run before cutting 0.4.0 -- no test covers it and `cargo test` passes on `main`'
Regression-test: 'not yet written -- a repository declaring `pointer.resolution: off` must not be blocked by `audit`, and a rule `audit` did not run must be reported `not-enabled`'
---
# 42 — `urzua audit` bypasses the rules table entirely

## What is wrong

`MILE-80` routed every rule in `check` through `gated()`, which skips a rule a repository has not
declared and replaces each finding's severity with the declared level. `audit` was not changed. It
calls `rules::pointer_resolution` and `rules::supersession_reciprocity` directly (`audit.rs:49-50`),
and contains **zero** `gated()` calls.

Three consequences, all of them the opposite of what `ADR-53` decided:

- A repository declaring `pointer.resolution: off` is still blocked by `audit`.
- A repository declaring it `warn` is still blocked by `audit`, because the declared level is ignored.
- A rule the repository never turned on is reported `status: "ran"`.

`ADR-53` says every rule is a policy and every policy is opt-in. `audit` opts a repository in without
asking, and there is no configuration that can decline it.

## Why it was missed

`MILE-80` was verified against `check`, the command whose report the measurements came from, and
`audit` shares the rule functions without sharing the gate. The test suite passes, because no test
runs `audit` against a config that declines a rule.

## Fix

`audit` calls `gated()` like `check` does. The gate belongs to the rule registry, not to one command,
and a third command reading rules directly would reintroduce this -- so the shared path is the fix,
not a second copy of the check.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** found by a review of the unreleased diff before cutting 0.4.0. `MILE-80` made every rule opt-in in `check` and left `audit` running two rules unconditionally at their hardcoded severity, so a repository cannot decline them by any configuration. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** `gated()` moved out of `check.rs` into `gate.rs`, and `audit` now routes both its rules through it. Shared rather than copied, because a third command reading rules directly would reintroduce this. Verified live: with `pointer.resolution: off`, `audit` reports `status: not-enabled, records_examined: 0` where it previously ran the rule at its hardcoded severity. | **substantive** |
