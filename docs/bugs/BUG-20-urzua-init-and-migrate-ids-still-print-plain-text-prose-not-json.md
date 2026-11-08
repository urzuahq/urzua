---
Stable-Id: 01M286MMQZFQ3S4THK6S2KCBCW
Status: Fixed
Found-in: 'implementing ADR-46''s Report/emit() contract -- checking every commands eprintln! guard clause found these two are plain text end to end, not just on their error paths'
Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082148a47756ba8d8a4b6e8dc7d85f66edb3748c63e7a7e5f01c96c, code:rust/crates/urzua-cli/src/commands/migrate.rs@sha256:772ecad047bcce8df4629aab9aa3ca993ee0968f5e255ec01500b096eadf444f, test:rust/crates/urzua-cli/tests/check_integration.rs#init_dry_run_reports_the_proposed_config_as_structured_json@sha256:a15d0e88686b90c34dbfc6fc6596fcba071f3d2bbfe605beca92f823b45a08e0, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_reports_per_file_outcomes_as_structured_json@sha256:1b28d251ff67a79e72a91daac97cd64243db7aaa8711f50fcd19ddb0bfeaddd8, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_exits_1_on_a_real_write_failure@sha256:7160c9cc1d09bf0982886b81221f99133846c51575a8b11c409d6b1097996c27, test:rust/crates/urzua-cli/tests/check_integration.rs#init_real_write_omits_config_yaml_from_the_report@sha256:1ca99d1a2261b9b2411e349bb9d4803a5fa33dc433fcae100ed550d444628b02
---
# 20 — `urzua init` and `migrate ids` still print plain-text prose, not JSON

## What was wrong

ADR-46 gave every other command (`check`, `audit`, `fix`, `new`, `explain`, `graph`, `doctor`,
`migrate schema --report`) one shared `Report`/`emit()` contract -- one JSON object on stdout,
unconditionally, success or failure. `urzua init` and `urzua migrate ids` were deliberately left out
of that migration, not overlooked: both are plain-text prose on every path, not just their error
guards. `run_init` prints sentences like `"urzua init: adopt mode -- proposing N record type(s):"`
followed by per-type lines and a `--dry-run` preview of the config file it would write; `run_migrate_
ids` prints `"urzua migrate ids: N record(s) missing a Stable-Id:"` followed by per-file `[OK]`/
`[SKIPPED]`/`[FAILED]` lines. Neither has a JSON success shape to extend -- wrapping only their error
paths in `CouldNotRun` while their success output stays prose would be a worse, half-migrated
inconsistency than leaving both alone.

ADR-23 itself already named `init`/`doctor` as not yet following the JSON contract when it shipped;
`doctor` has since caught up (confirmed directly: it already builds real `DoctorReport` JSON via
`emit()`). `init` and `migrate ids` are the two commands ADR-23's own gap notice was actually about,
still true today.

## Why nothing caught it

Nothing greps the `run_*` functions in `rust/crates/urzua-cli/src/main.rs` for a bare `println!`
outside a JSON-serializing call and flags it. Each command's output was reviewed in isolation when it
shipped, and `init`/`migrate ids`'s prose output looked reasonable on its own terms -- a human running
either directly in a terminal gets a readable summary. The gap only became visible once ADR-46 made
"every command shares one contract" a checkable, corpus-wide claim instead of an aspiration.

## References

- ADR-23 -- originally named `init`/`doctor` as not-yet-migrated; `doctor` closed its half, this bug
  tracked the other.
- ADR-46 -- the `Report`/`Notice`/`emit()` contract both commands now follow.
- `rust/crates/urzua-cli/src/commands/init.rs`/`migrate.rs` -- the fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial bug record, `Status: Open`. Not yet fixed -- whether the right shape is a `Report`-typed rewrite matching every other command, or a deliberately different convention for commands whose output is narrative by nature, is not yet decided. | **structural** |
> | 2026-09-11 | `Status: Fixed`. Built real `InitReport` (`proposed`/`written`/`config_toml`, the last only present on `--dry-run`, matching `new`'s own established convention of not echoing content back on a real write) and `MigrateIdsReport` (`missing`/`results`, each result a real `applied`/`skipped`/`failed` outcome with an error message where relevant). `migrate ids --apply` now also exits 1 on a real write failure, matching `fix --apply`'s `PartialFailure` signal for the same shape of outcome -- previously exit 0 unconditionally, a failed backfill visible only by reading the body. Landed alongside a mechanical split of `main.rs` (1392 lines, every command's logic and report type in one file) into one module per command under `commands/`, plus a `discovery.rs` for shared plumbing -- both new report types were written directly into their command's own module rather than added to the flat file first. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 4 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#init_dry_run_reports_the_proposed_config_as_structured_json` (lines 1063-1095) a2c8da3d1af5 -> a15d0e88686b, `rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_reports_per_file_outcomes_as_structured_json` (lines 1216-1249) 228b9cef1da5 -> 1b28d251ff67, `rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_exits_1_on_a_real_write_failure` (lines 1251-1298) 4f0287519b39 -> 7160c9cc1d09, `rust/crates/urzua-cli/tests/check_integration.rs#init_real_write_omits_config_yaml_from_the_report` (lines 1097-1117) 6f403b6af246 -> 1ca99d1a2261. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#init_dry_run_reports_the_proposed_config_as_structured_json: the test now also asserts the report status is partial-failure, agreeing with the exit code, and that the read-only record is left untouched, and the resolved span now runs on into the next test, migrate_ids_apply_refuses_while_another_apply_holds_the_lock; check_integration.rs#migrate_ids_apply_reports_per_file_outcomes_as_structured_json: the test now also asserts the report status is partial-failure, agreeing with the exit code, and that the read-only record is left untouched, and the resolved span now runs on into the next test, migrate_ids_apply_refuses_while_another_apply_holds_the_lock; check_integration.rs#migrate_ids_exits_1_on_a_real_write_failure: the test now also asserts the report status is partial-failure, agreeing with the exit code, and that the read-only record is left untouched, and the resolved span now runs on into the next test, migrate_ids_apply_refuses_while_another_apply_holds_the_lock; check_integration.rs#init_real_write_omits_config_yaml_from_the_report: the test now also asserts the report status is partial-failure, agreeing with the exit code, and that the read-only record is left untouched, and the resolved span now runs on into the next test, migrate_ids_apply_refuses_while_another_apply_holds_the_lock. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082, rust/crates/urzua-cli/src/commands/migrate.rs@sha256:772ecad047bc Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
