---
Stable-Id: 01M6D5VM1F4HBHKT51JKMX3HJZ
Status: Fixed
Found-in: "A /code-review pass, round 30"
Realized-by: test:rust/crates/urzua-cli/tests/locator_kinds_integration.rs#the_default_accepts_a_file_only_and_a_directory_or_glob_is_a_finding_naming_the_option_observed_failing@sha256:76886e71511391388b8565635b3545b6bb3243eb4e4526f2fc96d82b88c6cf3d, test:rust/crates/urzua-cli/tests/locator_kinds_integration.rs#an_opted_in_directory_or_glob_is_legal_when_it_matches_a_tracked_file_observed_failing@sha256:3945173205b165833178f6e3257369410905ae9eb8966e5b582bad21dd2cef2b, test:rust/crates/urzua-cli/tests/locator_kinds_integration.rs#a_directory_pin_hashes_the_sorted_paths_and_content_hashes_of_its_files_observed_failing@sha256:4bb8bd9ab735294504dedbadeb99d9ebd5735bfa2db6301b8a05416d0ce4a8ac
---
# 176 — whether a directory is a legal Realized-by locator is undecided and locator-exists rejects it as not in the working tree

## What was wrong

`Realized-by: code:src` where `src` is an existing directory is reported by `embodiment.locator-exists` as
"not in the working tree", and the message is also used for an untracked-but-on-disk file. Meanwhile git
pathspecs in drift detection accept a directory, so `embodiment.consistency` can compute drift for it.

## Why nothing caught it

`ADR-18` and `ADR-32` do not say whether a locator names a file or may name a tree. The check was written
for files.

## Why this needs a decision, not a mechanical fix

Needs a decision, not a mechanical fix: either a directory (and a glob) is a legal locator and the
existence check accepts a tracked prefix, or locators are files only and the message should say "not a
tracked file". Either way the wording "not in the working tree" asserts something the rule did not
establish for an existing directory or an untracked file.

## The decision

Maintainer decision, 2026-11-05: which locator kinds a `Realized-by` accepts, and which of those can
carry a pin, is configuration. `embodiment.locator-exists` takes `locator_kinds` and `pinnable`, each a
list of `file`, `directory` and `glob`, both defaulting to `[file]` (the behaviour before this fix). A
repository that opts in may cite a directory or glob, which is legal when it covers at least one
tracked file; drift uses the newest commit among the covered files; a pin hashes the sorted list of
(path, file content hash); `explain` shows the files; a locator of a kind not declared is a finding
that names the option. `SPEC-3` states the semantics.

## References

`BUG-175`. `ADR-74` (the locator grammar and pins), `ADR-73` (the two new finding variants declare
their suggested action), `MILE-158` (the glob matcher). Fixed in `embodiment_locator_exists`, the
drift check and `pins.rs`; `SPEC-2`, `SPEC-3`, `SPEC-8` and `SPEC-13` are updated. This repository
does not opt in to a kind: no record cites a directory or a glob.

## Regression test

planned: a rules.rs test pinning the decided behavior for Realized-by: code:<existing directory>

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed, left `Status: Open`. **Why:** found by a round 30 review and reproduced against a disposable fixture. | **substantive** |
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. With `Realized-by: code:src` where `src` is an existing tracked directory, `urzua check` reports `embodiment.locator-exists: Realized-by names 'src', which is not in the working tree`. **Why:** whether a locator may name a tree is the decision the record names. | **substantive** |
> | 2026-11-05 | `Status` moves from `Open` to `Fixed`. Maintainer decision, 2026-11-05: the accepted locator kinds and the pinnable kinds are the `locator_kinds` and `pinnable` options of `embodiment.locator-exists`, both defaulting to `[file]`; a directory or glob is legal when it covers a tracked file, drifts on the newest commit among those files and pins the hash of the sorted (path, file content hash) list. **Why:** the record asked for exactly this decision, and the default keeps today's behaviour for every repository that declares neither option. | **substantive** |
> | 2026-11-05 | `Regression-test` retired (ADR-84); the 3 tests that cover the change are now cited as `test:` locators in `Realized-by`, and the field is removed. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
