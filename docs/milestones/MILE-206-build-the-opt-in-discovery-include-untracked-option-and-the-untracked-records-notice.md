---
Stable-Id: 01M6SK9TM7F8FXEF7HCV4FD7AH
Status: Done
Phase: '0'
Track: schema-governance
Implements: ADR-92
Blocked-on: null
Realized-by: code:rust/crates/urzua-io/src/lib.rs, code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/report.rs, code:rust/crates/urzua-cli/src/discovery.rs, code:rust/crates/urzua-cli/src/commands/check.rs, code:rust/crates/urzua-cli/src/session.rs, code:rust/crates/urzua-cli/src/file_texts.rs, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/discovery_untracked_integration.rs, test:rust/crates/urzua-cli/tests/own_config_integration.rs, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-3-configuration.md
---
# 206 — Build the opt-in discovery.include_untracked option and the untracked-records notice

## What

Build `ADR-92`: a `discovery.include_untracked` config option that makes discovery include untracked,
non-ignored files under declared record directories, an info notice that names the untracked records a plain
`check` skipped, `--base` treating an included untracked record as added, and an info notice that drift did
not judge records with no history.

## Why

A record drafted and not yet staged is the one an author most needs `check` to read, and today a plain `check`
reports a clean run that did not read it (`BUG-24`, `ADR-56`). The maintainer decided on 2026-11-06 that a
repository may choose to govern untracked records and that the default must say what it skipped. It is Phase 0
because urzua governing its own repository in CI includes the records an author has written and not yet staged,
and the gate must not be clean about files it never read.

## Done means

- A `discovery` object with a boolean `include_untracked` (default `false`) loads, is documented in `SPEC-3`,
  and an unknown key under it is reported as for any other config object.
- With the option on, `discover_tracked_files` (whose signature and contract change: it gains config input and its doc comment is rewritten) also returns untracked, non-ignored files directly under a
  declared record directory that the type owns, and no file outside one; a file matched by `.gitignore` is
  not returned. `file_texts.rs`, `session.rs` and `init.rs` see the same set. Tests plant an untracked record,
  an ignored one and a scratch file outside every record directory, and assert the first only is examined.
- `check`'s `scope` has `include_untracked` and `untracked_included`; both are present with the option off.
- With the option off, a plain `check` over a tree with untracked records under declared directories reports an
  info notice `discovery.untracked-skipped` naming them (at most 20 paths and the count), emits none when there
  are none or when the file was named on argv, and does not change the exit code under `fail_on: warning`. A
  test fails before and passes after.
- `an_untracked_scratch_file_is_never_examined` and `an_untracked_file_below_an_explicitly_requested_directory_is_not_examined`
  still pass with the option off; with it on, a directory argument examines the untracked records below it.
- `check --base <ref>` with the option on reports an untracked record as added (in the changed and added
  sets) and judges it with the diff rules; with it off it behaves as before, including the refusal for a named
  untracked file outside what the option includes. Tests cover both.
- A record with no history is not judged for drift, and an info notice `discovery.untracked-not-drift-judged`
  gives the count of such records that carry a locator; a committed record is judged as before.
- `.urzua/config.yaml` sets `discovery.include_untracked: true`, and with an untracked record planted in
  `docs/` in a scratch copy `urzua check` examines it and `check --base origin/main` reports it as added.
  `urzua check` and `check --base origin/main` show 0 unwaived findings on the merged tree.
- `ADR-56` carries its dated `## Amendment (2026-11-06)` (`ADR-92` clause 9), and a reviewer confirms it still
  matches the built behavior.
- `SPEC-2`, `SPEC-3` and the notice table describe the option and both notices; `make records` and
  `make rule-table-check` pass.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-06 | Initial milestone. **Why:** maintainer decision of 2026-11-06 to add an opt-in untracked-records discovery option in Phase 0; implements `ADR-92`. | **structural** |
> | 2026-11-06 | Done means amended: `discover_tracked_files` changes signature and contract, and the `ADR-56` amendment is checked. `Status` unchanged. **Why:** independent review of 2026-11-06 of `ADR-92`, whose clauses 3 and 9 now state both. | **structural** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
