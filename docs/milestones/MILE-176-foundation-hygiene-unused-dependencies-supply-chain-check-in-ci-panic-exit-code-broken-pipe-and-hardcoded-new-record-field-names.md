---
Stable-Id: 01M6K40T93YGAFC0QNNK4F53XM
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: —
---
# 176 — Foundation hygiene: unused dependencies, supply-chain check in CI, panic exit code, broken pipe and hardcoded new-record field names

## What

A set of small hygiene fixes in the Rust workspace:

- remove unused dependencies: `anyhow` in `urzua-cli/Cargo.toml` (no use in its source), and `serde` and `serde_json` in `urzua-agdr` (only `thiserror` is used);
- add `cargo deny check` or `cargo audit` (and `cargo machete` for unused dependencies) to CI and to `make ci`; `yaml_serde` is a young crate with a single supplier;
- make a panic exit with the documented code 2 for "could not run" instead of Rust's default 101 (`urzua-cli/src/main.rs`, the panic hook near line 238);
- make `emit` write with `writeln!` and ignore `BrokenPipe` instead of `println!`, which panics on a closed pipe (`main.rs`, near line 42), with a test;
- replace the hardcoded `"Date"` and `"Author"` field names in `urzua-core/src/new_record.rs` (near lines 325 to 326) with a role or a per-field fill hint read from the type config.

## Why

Each is a small gap against a stated principle. The panic hook prints a JSON could-not-run result yet the process exits 101, so a caller reading the exit code sees a different contract than the docs state, and a closed pipe panics with the default code. The `new` write path guesses field names by literal, so a type that calls them `Created` and `Owner` gets `null` written to disk, which `ADR-64` (config in, structured out, no hardcoded type knowledge) rules out, and it relates to `MILE-137` and `MILE-151`. Unused dependencies and an absent supply-chain check are plain hygiene for a tool others will run in CI. Evidence read at the foundation review of 2026-11-04.

Findings 4 (fix drift cost) and 9 (git failures read as no drift) of that review are not filed: the drift gate, parallel blame and git-failure and shallow-clone notices landed in the `MILE-159` part 1 change (#214).

## Done means

No unused dependency remains and CI fails on one; a vulnerability check runs in CI and `make ci`; a panic exits 2 and a test asserts it; a closed stdout pipe exits without a panic and a test asserts it; `urzua new` fills date and author fields from the type's declaration and a fixture with differently named fields gets them filled; the capability is exercised by this repository's own config.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
