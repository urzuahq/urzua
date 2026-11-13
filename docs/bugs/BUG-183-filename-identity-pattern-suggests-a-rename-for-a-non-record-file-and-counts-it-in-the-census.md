---
Stable-Id: 01M6JKNYCZFWAQJ90KRWT13CBJ
Status: Fixed
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#a_declared_non_record_file_is_in_no_rule_population_or_census_observed_failing@sha256:0b663ddff969c9a2b0f118aab6f2bde4732ee5a1216cdaac60ae01c653974a68, test:rust/crates/urzua-cli/tests/check_integration.rs#an_excluded_file_is_not_counted_in_the_all_off_pattern_total@sha256:57390b3d1e2fda60f51a5a6f492cdd64c07eab755e3a24baf3c6bc4bf06bcd65, test:rust/crates/urzua-cli/tests/check_integration.rs#an_undeclared_stray_file_is_reported_beside_a_declared_exclusion@sha256:542db16c4d91cba61ad61891a79dd95473729376b1b5c7eda9a4a35fff43c086
---
# 183 — Filename.identity-pattern suggests a rename for a non-record file and counts it in the census

## What was wrong

`filename.identity-pattern` suggests renaming `README.md` to `SPEC-NNNN-README.md` and counts the README in its "none of the record type's N files match" total, treating an index file as a record that failed to match.

## Why nothing caught it

No fixture put a non-record file in a record directory, so the rule's file census was never tested against a file that is not a record. Related gap: the declaration excluding non-record files.

## References

BUG-3 of the adopter switchover report.

Fixed with `MILE-150`: a record type's `exclude_files` removes a named non-record file from record loading, so `filename.identity-pattern` neither flags nor counts it. See `SPEC-3` section on `exclude_files`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
> | 2026-11-04 | Fixed with `MILE-150`: a file the type declares in `exclude_files` is not loaded, so the rule neither suggests renaming it nor counts it. An undeclared stray file is still reported. `Regression-test` filled. **Why:** the rule's census was only ever fed records. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#a_declared_non_record_file_is_in_no_rule_population_or_census_observed_failing` (lines 3949-3970) 516bdbe39ed2 -> 0b663ddff969, `rust/crates/urzua-cli/tests/check_integration.rs#an_excluded_file_is_not_counted_in_the_all_off_pattern_total` (lines 4034-4051) f82319d361a7 -> 57390b3d1e2f, `rust/crates/urzua-cli/tests/check_integration.rs#an_undeclared_stray_file_is_reported_beside_a_declared_exclusion` (lines 3988-4007) 7c31e96882f1 -> 542db16c4d91. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#a_declared_non_record_file_is_in_no_rule_population_or_census_observed_failing: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; check_integration.rs#an_excluded_file_is_not_counted_in_the_all_off_pattern_total: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; check_integration.rs#an_undeclared_stray_file_is_reported_beside_a_declared_exclusion: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
