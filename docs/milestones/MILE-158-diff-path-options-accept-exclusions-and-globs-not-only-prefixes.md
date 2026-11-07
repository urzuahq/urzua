---
Stable-Id: 01M6JKNW1YB9CAQ7EMKFQE6HMW
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-core/src/config.rs#a_diff_path_option_takes_globs_and_refuses_a_broken_one@sha256:eb256a1c3fad0b6e92722e462acb5332389520f8f63e2b0574768717d7616759, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#a_diff_touching_only_test_files_beside_the_code_is_not_a_trigger@sha256:fd4b6b041cab5edc3291d0dc31934bbeb04f34ee48d284fb05338d1c4e53a4d0, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#a_prefix_only_configuration_still_treats_a_test_file_as_source@sha256:5cbe25362e28bb2772ce9842168359f48b98586a98e04ff511d9106b3df277cf, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#code_changed_beside_its_tests_still_triggers_and_names_the_code@sha256:284d42f6fc3ceb0f6f905d44dec4901cdb89a6ea3231e3fd26ad0b82da83edd2, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#this_repositorys_own_config_lets_a_crate_test_change_through_but_not_its_source@sha256:90f69a5dff0797ad619620f85343a0e5a0fec236a9ce4d1d5bd379c7d797895a
---
# 158 — Diff path options accept exclusions and globs, not only prefixes

## What

Let the `diff.*` path options take globs and exclusions so a set such as every source file except test files can be stated.

## Why

The options are prefixes only, so when tests sit beside the code in `*.test.*` files there is no way to say everything except tests. The rule `diff` is meant to answer "did code change without a governing record"; a test-only change must not trip it, and the configuration cannot say so. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A path option may declare an exclusion or glob. — evidence: test:rust/crates/urzua-core/src/config.rs#a_diff_path_option_takes_globs_and_refuses_a_broken_one@sha256:eb256a1c3fad0b6e92722e462acb5332389520f8f63e2b0574768717d7616759
- A diff touching only an excluded file is not a trigger. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#a_diff_touching_only_test_files_beside_the_code_is_not_a_trigger@sha256:fd4b6b041cab5edc3291d0dc31934bbeb04f34ee48d284fb05338d1c4e53a4d0
- Prefix-only configurations behave unchanged. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#a_prefix_only_configuration_still_treats_a_test_file_as_source@sha256:5cbe25362e28bb2772ce9842168359f48b98586a98e04ff511d9106b3df277cf
- Fixtures cover tests beside code. — evidence: test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#code_changed_beside_its_tests_still_triggers_and_names_the_code@sha256:284d42f6fc3ceb0f6f905d44dec4901cdb89a6ea3231e3fd26ad0b82da83edd2
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#this_repositorys_own_config_lets_a_crate_test_change_through_but_not_its_source@sha256:90f69a5dff0797ad619620f85343a0e5a0fec236a9ce4d1d5bd379c7d797895a

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: the diff rules' path options accept globs and `exclude_paths` also narrows `diff.source-requires-record`, leaving excluded paths eligible and unexamined. This repository excludes `rust/crates/*/tests` from `source_paths`, and `diff_gate_integration.rs` runs the shipped config on a test-only and a source change. `Status` stays `Planned`. **Why:** the Done-when contract; status is a human decision. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #200: the diff rules' path options accept globs and `exclude_paths`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
