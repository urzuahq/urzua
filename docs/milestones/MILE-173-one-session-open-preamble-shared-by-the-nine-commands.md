---
Stable-Id: 01M6K40RCPZCY5R863JS9SQ766
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/session.rs, code:rust/crates/urzua-cli/src/commands/audit.rs, code:rust/crates/urzua-cli/src/commands/check.rs, code:rust/crates/urzua-cli/src/commands/explain.rs, code:rust/crates/urzua-cli/src/commands/export.rs, code:rust/crates/urzua-cli/src/commands/fix.rs, code:rust/crates/urzua-cli/src/commands/graph.rs, code:rust/crates/urzua-cli/src/commands/migrate.rs, code:rust/crates/urzua-cli/src/commands/new.rs, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_corpus_command_reports_a_missing_config@sha256:40190f1b0e647b193a4ed64de5d9e470c3de083a0de86a440fb9801617aac94e, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_corpus_command_reports_an_unreadable_tracked_record@sha256:476efc79f5684fc420111c35aa187623abef225e42a491c49e4de918c0694da1
---
# 173 — One Session open preamble shared by the nine commands

## What

Add one `Session::open(config_path) -> Result<Session, String>` holding the repo root, config, discovered files and loaded records, and one wrapper that turns a `Result` into the could-not-run emission, then use it in all nine commands: `audit`, `check`, `doctor`, `explain`, `export`, `fix`, `graph`, `migrate` and `new` (`urzua-cli/src/commands/*.rs`).

## Why

Each command repeats about 25 lines: `find_repo_root`, `default_config_path` and `load_config`, `discover_tracked_files`, `load_records`, with a `match ... return emit(&CouldNotRun::from(e))` arm for each, because the functions return `ExitCode` and cannot use `?`. Any change to discovery order, the not-run exit code or the error text is nine edits, and a missed one is a silent divergence. This is the shape that produced `BUG-42` (`audit` running rules directly and bypassing the table), and `fix` carries its own copy of check's setup that can disagree about gating. `SPEC-22`'s extract-on-second-occurrence rule applies nine times over. The `Result<_, String>` convention is right; the change is mechanical. Evidence read at the foundation review of 2026-11-04.

Ordering: independent of `MILE-171`; do it first if a small change is wanted, since it shrinks the surface `MILE-172` has to split.

## Done means

- All nine commands open through `Session::open` and report a could-not-run error through one wrapper; no command repeats the load sequence. — evidence: code:rust/crates/urzua-cli/src/session.rs, code:rust/crates/urzua-cli/src/commands/audit.rs, code:rust/crates/urzua-cli/src/commands/check.rs, code:rust/crates/urzua-cli/src/commands/explain.rs, code:rust/crates/urzua-cli/src/commands/export.rs, code:rust/crates/urzua-cli/src/commands/fix.rs, code:rust/crates/urzua-cli/src/commands/graph.rs, code:rust/crates/urzua-cli/src/commands/migrate.rs, code:rust/crates/urzua-cli/src/commands/new.rs
- A test shows each command exits with the documented could-not-run code for a missing config, a missing repo and an unreadable record. — evidence: test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_corpus_command_reports_a_missing_config@sha256:40190f1b0e647b193a4ed64de5d9e470c3de083a0de86a440fb9801617aac94e, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_corpus_command_reports_an_unreadable_tracked_record@sha256:476efc79f5684fc420111c35aa187623abef225e42a491c49e4de918c0694da1
- The same error text is reported for a missing config, a missing repo and an unreadable record across the commands. — evidence: test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_corpus_command_reports_a_missing_config@sha256:40190f1b0e647b193a4ed64de5d9e470c3de083a0de86a440fb9801617aac94e, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_corpus_command_reports_an_unreadable_tracked_record@sha256:476efc79f5684fc420111c35aa187623abef225e42a491c49e4de918c0694da1
- Behaviour and JSON output are otherwise unchanged.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
> | 2026-11-04 | Delivered in full; `Status: Done` with `Realized-by`. `audit`, `explain`, `export`, `graph` and both `migrate` subcommands open through `Session::open` and return `Result<ExitCode, String>` from an inner function wrapped by `emit_or_not_run`. `check` (`Session::open_with`, its argv-override and `--base` steps as the adjust closure), `fix` (`Session::open`) and `new` (`Workspace::open`) take the same preamble but keep their many other could-not-run arms in place, so their bodies stay `ExitCode`-returning; `index` and `refs refresh`, which repeated the same preamble, use `Workspace`/`Session::load` too. `doctor` keeps its own root lookup because a missing or broken config is a report of its own, not a could-not-run. `could_not_run_integration.rs` pins the exit code and exact JSON of a missing repository, a missing config and an unreadable tracked record for every command, and was written and passing before the refactor. **Why:** the Done-when. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
