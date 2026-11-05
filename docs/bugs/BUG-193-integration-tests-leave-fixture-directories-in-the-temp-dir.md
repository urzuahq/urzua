---
Stable-Id: 01M6N3YAVAVBXWJXACN8G1NS6B
Status: Fixed
Found-in: 'the disk filling after tens of thousands of test runs; every urzua-* fixture directory (urzua-pins-*, urzua-cli-test-*, urzua-types-* and others) was still in the temp dir (2026-11-05)'
Realized-by: code:rust/crates/urzua-cli/tests/common/mod.rs, test:.github/scripts/test-no-leak.test.sh, code:.github/scripts/test-no-leak.sh
---
# 193 — Integration tests leave fixture directories in the temp dir on every run

## What was wrong

Each integration test built its fixture under `std::env::temp_dir()` as `urzua-<kind>-<name>-<pid>`, and most never removed it; the few that did removed it with a trailing `remove_dir_all` that a failing assertion skipped. A full run left about 285 directories behind, each named by the process id, so no later run reused them. Across thousands of local and agent runs they filled the disk.

A second defect of the same shape: the `sha256` helper in `evidence_pins_integration.rs` named its scratch file from the process id and the content length, so two parallel tests hashing same-length content wrote and removed one file. That made `a_pin_that_no_longer_matches_is_drift_even_when_the_record_is_newer_observed_failing` flaky in CI.

## Why nothing caught it

Nothing measured what a run leaves behind: every test passed whether or not its directory survived, and the leak is invisible in one run. Cleanup was a hand-written last line of each test rather than something owned by the fixture, so it ran only on the success path and was missing from most tests.

## References

Fixed with a `TempDir` guard (`rust/crates/urzua-cli/tests/common/mod.rs`; a small local guard in `urzua-io`'s unit tests) that removes the directory on drop, including on panic. Names are `<label>-<pid>-<n>` with a per-process counter, so no two fixtures or scratch files share a name; `TempFile` replaces the colliding hash file. `.github/scripts/test-no-leak.sh` runs the suite under a fresh `TMPDIR` and fails when any `urzua-*` entry remains; `checks.yml` and `make ci` run it, and `SPEC-20` is updated. `tempfile` was not already a dev-dependency, so the guard is hand-written.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed and fixed. **Why:** the maintainer reported the temp dir filling with fixture directories; a fixture's cleanup must not depend on its test passing. | **substantive** |
> | 2026-11-05 | `Regression-test` retired (ADR-84); the guard's own test and the script that runs the suite under a fresh `TMPDIR` are cited in `Realized-by`, and the field is removed. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
