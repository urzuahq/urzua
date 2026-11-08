---
Stable-Id: 01M6JKNZNG70E53R1KHPCY40MW
Status: Fixed
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#a_config_level_finding_names_a_repository_relative_config_path_observed_failing@sha256:c99cb628975b7ba8dffc0118b9eea84b33feae5372ce8480f6e68ab27f4681d2
---
# 185 — Config.scope-matches-nothing reports its file as an absolute path

## What was wrong

`config.scope-matches-nothing` reports its `file` as the absolute path to the configuration, while every other rule reports a repository-relative path. Minor: output differs by machine, so a finding cannot be compared or baselined across checkouts.

## Why nothing caught it

The rule's tests asserted the message, never the `file` field's form, so the one rule that built its path differently was not noticed.

## References

BUG-5 of the adopter switchover report.

Fixed in `check`: a finding whose file is the loaded configuration is reported relative to the repository root, which covers every config-level rule, not only this one.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
> | 2026-11-04 | Fixed: `check` reports a finding about the configuration as a path relative to the repository root, after the scope filters and before waivers match. Fixed for every config-level rule at once, since all received the same absolute path. `Regression-test` filled. **Why:** the path was built from the repository root and never relativized. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#a_config_level_finding_names_a_repository_relative_config_path_observed_failing` (lines 4053-4072) 0e71d4999b62 -> c99cb628975b. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#a_config_level_finding_names_a_repository_relative_config_path_observed_failing: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
