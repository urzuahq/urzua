---
Stable-Id: 01M6JKNYCZFWAQJ90KRWT13CBJ
Status: Fixed
Found-in: 'a trial run over a real adopter corpus (2026-11-04)'
Regression-test: 'rust/crates/urzua-cli/tests/check_integration.rs::a_declared_non_record_file_is_in_no_rule_population_or_census_observed_failing, rust/crates/urzua-cli/tests/check_integration.rs::an_excluded_file_is_not_counted_in_the_all_off_pattern_total, rust/crates/urzua-cli/tests/check_integration.rs::an_undeclared_stray_file_is_reported_beside_a_declared_exclusion'
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
