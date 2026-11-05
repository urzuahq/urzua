---
Stable-Id: 01M6JD5KJ8BYS9GRKN1AX26B1F
Status: Fixed
Found-in: 'enabling fail_on: warning (MILE-145); urzua check --base origin/main reported a config.scope-matches-nothing warning on a diff touching a Rust source and a record (2026-11-04)'
Realized-by: test:rust/crates/urzua-core/src/rules/back_pointer.rs#a_file_the_diff_did_not_change_is_not_out_of_scope_observed_failing@sha256:919212b47610154c4ab406580db4cce347b3691f8b68390c9a30b57c7f4e6c9a, test:rust/crates/urzua-core/src/rules/back_pointer.rs#a_changed_file_outside_the_declared_paths_is_still_out_of_scope@sha256:ed43e7fa383bed8a059593d23ca21c76e6d661c32192311175710d71aeb035f5
---
# 180 — A --base run reports unchanged source files as out of scope so config.scope-matches-nothing warns on every pull request

## What was wrong

On a `urzua check --base <ref>` run, `embodiment.back-pointer-decision-accepted` counted every source
file the diff had not changed as `OutOfScope`, the outcome that means the configuration cannot reach
the file. `config.scope-matches-nothing` read that and warned that the rule "was handed 96
candidate(s) the configuration does not reach", on every pull request whose diff touched a source
file and a record. The warning is false: the files are inside `back_pointer_paths`, and the diff
narrowing is not a configuration fault. It stayed invisible because a warning exits 0.

## Why nothing caught it

The `--base` tests asserted which back-pointers were judged, never the rule's population, and no
check ran with warnings counted as failures until `fail_on: warning` (`MILE-145`) made this
repository's pull-request gate do so.

## References

Fixed in `embodiment_back_pointer_decision_accepted`: the diff now narrows the candidate list before
the census, so an unchanged file is not a candidate, while a changed file outside the declared paths
is still out of scope. Found under `MILE-145`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and fixed. **Why:** `MILE-145` made warnings fail the pull-request gate, which exposed a warning that every such pull request produced. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
