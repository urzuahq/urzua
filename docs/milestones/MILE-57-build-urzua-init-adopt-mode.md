---
Status: Done
Stable-Id: 01M1YNN9K5B2V3D7MSGD9HWNG1
Phase: '0'
Track: roadmap-tracking
Implements: SPEC-5
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082148a47756ba8d8a4b6e8dc7d85f66edb3748c63e7a7e5f01c96c, test:rust/crates/urzua-cli/tests/init_adopt_integration.rs#init_on_a_realistic_repo_writes_a_config_that_loads_with_a_sensible_type_count@sha256:c0f4c016e42880449f40aa2cd35265af35a73a3c08f6b04838c6b98ff88437fc, test:rust/crates/urzua-cli/tests/check_integration.rs#init_then_check_names_a_pre_existing_blockquote_record_as_an_unsupported_form@sha256:1b223594d7badb7448e36b5647e11808f59aa675ca9bdfd2bb694aedff0cce4c, test:rust/crates/urzua-cli/tests/check_integration.rs#init_flags_headerless_numbered_pages_and_finds_a_slug_named_record_directory@sha256:9dfab385e44a3154db84289822aacd1f6be2073bf9146afb8a5fc27f8476a523, test:rust/crates/urzua-cli/tests/check_integration.rs#a_post_disambiguation_name_collision_is_refused_not_silently_dropped_observed_failing@sha256:579c7e4c0643234d795a0b024db742b95d5f9cb6cb5d234a0bd1f2e1ded6929a, test:rust/crates/urzua-cli/tests/init_adopt_integration.rs#init_names_the_directories_it_did_not_propose@sha256:4dba85f30e5010d017c10312c58c42505de73e5f534b3f40b11666a739f0f7c7
---
# 57 — Build urzua init adopt mode

## What

`urzua init` adopt mode: proposes `.urzua/config.toml` from an existing corpus's real directories and header conventions, never moving or rewriting a single record. `--dry-run` byte-identical to the real run.

## Why

The tool has to be adoptable without a rewrite for "one engine instead of a fork per repo" to mean anything -- adopt mode is the only path that doesn't ask an existing corpus to change shape first.

## Done means

- `urzua init` adopt mode proposes a config from an existing corpus's real directories and header conventions. — evidence: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082148a47756ba8d8a4b6e8dc7d85f66edb3748c63e7a7e5f01c96c, test:rust/crates/urzua-cli/tests/init_adopt_integration.rs#init_on_a_realistic_repo_writes_a_config_that_loads_with_a_sensible_type_count@sha256:c0f4c016e42880449f40aa2cd35265af35a73a3c08f6b04838c6b98ff88437fc
- `urzua init` never moves or rewrites a record. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#init_then_check_names_a_pre_existing_blockquote_record_as_an_unsupported_form@sha256:1b223594d7badb7448e36b5647e11808f59aa675ca9bdfd2bb694aedff0cce4c
- `--dry-run` output is byte-identical to the real run's. — evidence: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082148a47756ba8d8a4b6e8dc7d85f66edb3748c63e7a7e5f01c96c
- A corpus outside `docs/` or with a colliding type name is adopted or refused with a message rather than silently dropped. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#init_flags_headerless_numbered_pages_and_finds_a_slug_named_record_directory@sha256:9dfab385e44a3154db84289822aacd1f6be2073bf9146afb8a5fc27f8476a523, test:rust/crates/urzua-cli/tests/check_integration.rs#a_post_disambiguation_name_collision_is_refused_not_silently_dropped_observed_failing@sha256:579c7e4c0643234d795a0b024db742b95d5f9cb6cb5d234a0bd1f2e1ded6929a, test:rust/crates/urzua-cli/tests/init_adopt_integration.rs#init_names_the_directories_it_did_not_propose@sha256:4dba85f30e5010d017c10312c58c42505de73e5f534b3f40b11666a739f0f7c7
- The written config is `.urzua/config.yaml`, which supersedes the `.toml` named in the What. — evidence: test:rust/crates/urzua-cli/tests/init_adopt_integration.rs#init_on_a_realistic_repo_writes_a_config_that_loads_with_a_sensible_type_count@sha256:c0f4c016e42880449f40aa2cd35265af35a73a3c08f6b04838c6b98ff88437fc

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082 Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs, config.rs and runner.rs still carry the info level (MILE-46), init.rs still proposes a config in adopt mode (MILE-57), doctor.rs, the Makefile, both git hooks and checks.yml still report and wire the gate (MILE-58), waiver.rs, the waiver template and the WAIVER-0035 record still define and hold a reviewed exception (MILE-59), new.rs and new_record.rs still create the next record (MILE-60), both fix files and fix_drift_integration.rs still detect and apply (MILE-61), pointer.rs, audit.rs and the corpus-b expectation still run the audit that reports one-sided supersession (MILE-62), explain.rs and graph.rs still print the governing records and the edges (MILE-63), the corpus-a expectation still shows a config naming the removed bold-list shape rejected, which is what the milestone's item says (MILE-65), and migrate.rs still reports what a newly required field would break without writing (MILE-68). MILE-46's revision table had a blank line between its header and its rows, which hid every row from the parser, so that line was removed before pinning. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
