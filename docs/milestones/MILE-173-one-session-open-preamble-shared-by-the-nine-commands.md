---
Stable-Id: 01M6K40RCPZCY5R863JS9SQ766
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: —
---
# 173 — One Session open preamble shared by the nine commands

## What

Add one `Session::open(config_path) -> Result<Session, String>` holding the repo root, config, discovered files and loaded records, and one wrapper that turns a `Result` into the could-not-run emission, then use it in all nine commands: `audit`, `check`, `doctor`, `explain`, `export`, `fix`, `graph`, `migrate` and `new` (`urzua-cli/src/commands/*.rs`).

## Why

Each command repeats about 25 lines: `find_repo_root`, `default_config_path` and `load_config`, `discover_tracked_files`, `load_records`, with a `match ... return emit(&CouldNotRun::from(e))` arm for each, because the functions return `ExitCode` and cannot use `?`. Any change to discovery order, the not-run exit code or the error text is nine edits, and a missed one is a silent divergence. This is the shape that produced `BUG-42` (`audit` running rules directly and bypassing the table), and `fix` carries its own copy of check's setup that can disagree about gating. `SPEC-22`'s extract-on-second-occurrence rule applies nine times over. The `Result<_, String>` convention is right; the change is mechanical. Evidence read at the foundation review of 2026-11-04.

Ordering: independent of `MILE-171`; do it first if a small change is wanted, since it shrinks the surface `MILE-172` has to split.

## Done means

All nine commands open through `Session::open` and report a could-not-run error through one wrapper; no command repeats the load sequence; a test shows each command exits with the documented could-not-run code and the same error text for a missing config, a missing repo and an unreadable record; behaviour and JSON output are otherwise unchanged.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
