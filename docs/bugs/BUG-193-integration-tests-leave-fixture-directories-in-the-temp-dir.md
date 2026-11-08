---
Stable-Id: 01M6N3YAVAVBXWJXACN8G1NS6B
Status: Fixed
Found-in: 'the disk filling after tens of thousands of test runs; every urzua-* fixture directory (urzua-pins-*, urzua-cli-test-*, urzua-types-* and others) was still in the temp dir (2026-11-05)'
Realized-by: code:rust/crates/urzua-cli/tests/common/mod.rs@sha256:1947a4dc3d3ee4eb21d686d03d5f59e3001c0e1519a542e3ebc98dc42f48a4a8, test:.github/scripts/test-no-leak.test.sh@sha256:36c44d3cd4c56c6483e31d10e31ce44b29db039518446a3f305a1f1f4a2bce9d, code:.github/scripts/test-no-leak.sh@sha256:a3bb37ffebf0db8cd7c46299196c9c33be65c598fa814fbb8a337bff42558f5e
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
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/tests/common/mod.rs@sha256:1947a4dc3d3e, .github/scripts/test-no-leak.test.sh@sha256:36c44d3cd4c5, .github/scripts/test-no-leak.sh@sha256:a3bb37ffebf0 Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
