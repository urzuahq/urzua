---
Stable-Id: 01M6JW43RMC4HFW0TD4PBBVFZS
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_glob_test_path_and_a_prefix_work_together_and_an_unmatched_entry_is_reported@sha256:44b405239ec6f65ebd42ed91d62faaeb026f846600a6f3add413ee9f1747893d, code:rust/crates/urzua-core/src/rules/back_pointer.rs, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-embodiment-rules-run.yaml
---
# 160 — `test_paths` accepts globs, not only prefixes

## What

Let the `test_paths` setting (`MILE-146`) take glob patterns such as `**/*.test.*` as well as directory prefixes.

## Why

Test files in a real adopter corpus sit beside the code they test and are named by suffix, not collected under one directory, so a prefix list cannot name them. `MILE-158` gave the diff path options globs for the same reason. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A `test_paths` entry that contains a glob metacharacter is matched as a glob and any other entry stays a prefix. — evidence: test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_glob_test_path_and_a_prefix_work_together_and_an_unmatched_entry_is_reported@sha256:44b405239ec6f65ebd42ed91d62faaeb026f846600a6f3add413ee9f1747893d, code:rust/crates/urzua-core/src/rules/back_pointer.rs
- An entry that matches no file is reported rather than silently ignored. — evidence: test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_glob_test_path_and_a_prefix_work_together_and_an_unmatched_entry_is_reported@sha256:44b405239ec6f65ebd42ed91d62faaeb026f846600a6f3add413ee9f1747893d
- A fixture covers a prefix and a glob together. — evidence: test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_glob_test_path_and_a_prefix_work_together_and_an_unmatched_entry_is_reported@sha256:44b405239ec6f65ebd42ed91d62faaeb026f846600a6f3add413ee9f1747893d
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-embodiment-rules-run.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: `test_paths` entries with a glob character are matched by the diff rules' matcher and others stay prefixes; a glob matching no scanned file is reported and one that cannot compile is refused at load. This repository's config declares a prefix and the glob `rust/crates/*-core/tests`, and `test_tier_integration.rs` covers both together. `Status` stays `Planned`. **Why:** the Done-when contract. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #209: `test_paths` accepts globs beside prefixes. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
