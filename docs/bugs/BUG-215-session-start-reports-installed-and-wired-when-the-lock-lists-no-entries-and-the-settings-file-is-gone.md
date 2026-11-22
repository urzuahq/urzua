---
Stable-Id: 01M82KCT2DQR0GRQEPPXR6BSFW
Status: Fixed
Found-in: independent review of pull request #622 (MILE-354), 2026-11-22: reproduced with the lock's `entries` set to `[]` and `.claude/settings.json` deleted; session-start returned `allow`.
Realized-by: test:rust/crates/urzua-cli/tests/harness_integration.rs#session_start_names_a_missing_settings_file_when_the_lock_lists_no_entries@sha256:51e84e6ed7f9c2480a7d147c8e8de816e92047af0765f7b877a5680114967607
---
# 215 — Session start reports installed and wired when the lock lists no entries and the settings file is gone

## What was wrong

`hook_entry_problems` (`rust/crates/urzua-cli/src/harness/run.rs`) names a hook entry that is not wired by walking the lock's entries, then walks the entries the bundle wants that the lock does not list. The second walk skipped a file whose settings could not be read, on the premise that the first walk had already named every locked entry. When the lock lists no entry for that file there is nothing to name, so session-start returned `allow` ("installed and wired") for a repository whose settings file was missing and wired nothing. It now names the file once. A file the lock does list entries for is still named by the locked-entry walk only, so nothing is reported twice.

## Why nothing caught it

No test covered a lock with no entries for a missing settings file; the existing test removed one entry and kept the others, so the locked-entry walk always had something to report.

## References

`session_start_names_a_missing_settings_file_when_the_lock_lists_no_entries` in `rust/crates/urzua-cli/tests/harness_integration.rs`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-22 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/tests/harness_integration.rs#session_start_names_a_missing_settings_file_when_the_lock_lists_no_entries@sha256:51e84e6ed7f9 Reason: Pin the new test that plants a lock with no entries and a missing settings file **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-22 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Open` disagreed with the computed value `Fixed`. | **structural** |
