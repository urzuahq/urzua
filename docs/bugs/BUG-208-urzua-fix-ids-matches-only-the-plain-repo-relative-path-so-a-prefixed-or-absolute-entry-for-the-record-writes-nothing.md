---
Stable-Id: 01M7SKPQVW3VJM50B5WS228AR7
Status: Fixed
Found-in: the same run, ./docs/adr/ADR-2-old.md, an absolute path and a path that names no pending repair were each reported as matching no pending repair
Realized-by: test:rust/crates/urzua-cli/tests/fix_exit_codes_integration.rs#a_dot_slash_ids_entry_writes_the_repair@sha256:f044bd162f5adb16ae97e9ce83688fef98893d6a61ab003c2c0f736fe3ff6b68, test:rust/crates/urzua-cli/tests/fix_exit_codes_integration.rs#an_absolute_ids_entry_inside_the_repository_writes_the_repair@sha256:42351c27a9f825ac31508e6e05328fd92e9aff6f09230369684ef53063ad56cd, test:rust/crates/urzua-cli/tests/reverify_integration.rs#a_dot_slash_ids_entry_reviews_the_record_like_its_plain_path@sha256:81e29a7fe7efb7e71e6dc44fb330cc097fd50de3fc6c8b55f30ad54e4f6e5e3c, test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_dot_slash_ids_entry_resolves_the_conflicted_record@sha256:b22764154e0c5cee18bb5ee3f4f547ceaf781b647fdd1d5df6dde0a0f8cb0c4d, test:rust/crates/urzua-cli/tests/fix_exit_codes_integration.rs#a_dot_slash_ids_entry_selects_the_stale_pin_for_a_re_pin@sha256:1a27cdc67a5e6043d30a42f6fd01541f5f59a6026cd6e72cbceb7371b6424a83
---
# 208 — urzua fix --ids matches only the plain repo-relative path, so a ./-prefixed or absolute entry for the record writes nothing

## What was wrong

`urzua fix --ids` compared each entry as a string with a record's repo-relative path, so `./docs/adr/ADR-2-old.md`, or the same path written absolute inside the repository, matched no pending repair and `--apply` wrote nothing, though the record was the one the editor hook had named. This held for every mode that takes `--ids`: plain, `--rebaseline`, `--pin` and `--conflicts`.

## Why nothing caught it

Every existing test passed `--ids` in the one plain form, so the other spellings of the same path were never exercised; `check <PATHS>` and `--stdin-path` already normalised their paths and `--ids` did not share that code.

## References

Fixed in `rust/crates/urzua-cli/src/commands/fix.rs` (`repo_relative_ids`, used by `fix_pin.rs`, `fix_conflicts.rs` and `reverify.rs` too), which reuses `discovery::relative_to_root` for an absolute path. An absolute path outside the repository is kept as written and fails naming it. `SPEC-8` and `docs/reference/commands.md` state it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-19 | Review of the pull request found `reverify --ids` matched only the plain path as well, and that no test covered `--rebaseline` or `--conflicts` with a `./`-prefixed entry. `reverify` now uses the same normaliser; `Realized-by` gains a test for each of `reverify`, `--conflicts` and `--rebaseline`. | **structural** |
