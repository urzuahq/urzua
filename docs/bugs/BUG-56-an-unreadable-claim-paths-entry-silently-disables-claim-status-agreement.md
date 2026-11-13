---
Stable-Id: 01M2W328KQGTSR7CGJ2T1CY52T
Status: Fixed
Found-in: 'A cumulative code review of v0.3.0..release -- reproduced with a typo of .changeset'
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#a_claim_paths_entry_naming_a_file_does_not_load@sha256:f44cc84357b45888a6a7762e7835a156dcc7bf1ba2990f901cc9b256ec7b510f, test:rust/crates/urzua-cli/tests/check_integration.rs#an_absent_claim_paths_directory_is_reported_not_fatal@sha256:578f632f94beb565a5c4e48cfd7ca266adaad9e6117865413e71b753a130626d
Blocked-on: —
---
# 56 — An unreadable `claim_paths` entry silently disables `claim.status-agreement`

## What was wrong

`read_claim_files` swallowed an unresolvable directory with `let Ok(entries) = read_dir(..) else
{ continue };`. A config saying `claim_paths: [".changesets"]` -- one letter off -- produced
`status: ok`, `blocking: false`, zero findings, and `rules_executed` reporting
`{status: "ran", records_examined: 0}`, against a corpus that genuinely contained a false close claim.

Indistinguishable from a clean run. `BUG-44` added `OptionRequired` precisely to stop this rule going
inert when `claim_paths` is *missing*; a misspelled one went through the same door.

## Fix (shipped)

Validated before any rule runs, because it is a config error rather than a finding about a record:
a `claim_paths` entry that is not a readable directory exits `not-run` and names it.

Verified: the typo'd config now reports
*"claim.status-agreement: claim_paths entry '.changesets' is not a readable directory"*.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed and fixed. **Why:** a rule that cannot reach its input reported success, which is this project's recurring failure in the one rule written to catch a false claim. | **substantive** |
> | 2026-09-22 | Regression tests written, and the fix narrowed. **Why:** this bug asked that an unresolvable `claim_paths` entry not be *indistinguishable from a clean run*. Aborting achieved that and took `check` down with a directory git cannot keep -- `.changeset` ceases to exist the moment a release consumes the last fragment, so a repository following the documented pattern lost `check` entirely. Absence is now a `Notice`, which is visible and never moves the exit code (`ADR-46`), so the requirement is met without the collateral. A path that is actively wrong -- a file where a directory was declared, or a link out of the repository -- still aborts. Two abort sites had to be fixed: the guard and the reader, which failed independently with different messages. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#an_absent_claim_paths_directory_is_reported_not_fatal` (lines 2792-2832) 9a260af8a0b5 -> 578f632f94be. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#an_absent_claim_paths_directory_is_reported_not_fatal: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
