---
Stable-Id: 01M286MMQZFQ3S4THK6S2KCBCW
Status: Fixed
Found-in: 'implementing ADR-46''s Report/emit() contract -- checking every commands eprintln! guard clause found these two are plain text end to end, not just on their error paths'
Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs, code:rust/crates/urzua-cli/src/commands/migrate.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#init_dry_run_reports_the_proposed_config_as_structured_json@sha256:a2c8da3d1af56bf927e8f09b01bcb64f586e3b79daae2eec3b7d71d9d4b42b26, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_reports_per_file_outcomes_as_structured_json@sha256:228b9cef1da50bfa61fcfa4003ebdfd8b77eaa6df0241987ddc32e828b5f06a1, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_exits_1_on_a_real_write_failure@sha256:4f0287519b39665b32f7e8933581281845d152e25383e188bab8f73d51e77f0b, test:rust/crates/urzua-cli/tests/check_integration.rs#init_real_write_omits_config_yaml_from_the_report@sha256:6f403b6af2469884629a62b69fbc8a268bbf0ee8e4a43fcf818acc3854f04785
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
