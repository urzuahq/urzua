---
Stable-Id: 01M6K40T93YGAFC0QNNK4F53XM
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: —
Realized-by: code:.github/workflows/checks.yml, code:rust/deny.toml, code:Makefile, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_panic_exits_2_with_a_not_run_result@sha256:ed00fea0578930cb2a31aaa00e999fee926b483847dbbe029115ec6954d5e6ca, test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_closed_stdout_does_not_panic@sha256:6610198fac3875185be86ba8e8eac415d7e92e38a91a26320f924b0d9cca6cfb, code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_differently_named_date_and_author_fields_from_the_declaration@sha256:21637897a097da1db8c5af427984af8a6077f9e5a6a43a28934ff9c8672dea6d, code:.urzua/config.yaml
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

- No unused dependency remains and CI fails on one. — evidence: code:.github/workflows/checks.yml
- A vulnerability check runs in CI and `make ci`. — evidence: code:rust/deny.toml, code:.github/workflows/checks.yml, code:Makefile
- A panic exits 2 and a test asserts it. — evidence: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_panic_exits_2_with_a_not_run_result@sha256:ed00fea0578930cb2a31aaa00e999fee926b483847dbbe029115ec6954d5e6ca
- A closed stdout pipe exits without a panic and a test asserts it. — evidence: test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_closed_stdout_does_not_panic@sha256:6610198fac3875185be86ba8e8eac415d7e92e38a91a26320f924b0d9cca6cfb
- `urzua new` fills date and author fields from the type's declaration and a fixture with differently named fields gets them filled. — evidence: code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_differently_named_date_and_author_fields_from_the_declaration@sha256:21637897a097da1db8c5af427984af8a6077f9e5a6a43a28934ff9c8672dea6d
- The capability is exercised by this repository's own config. — evidence: code:.urzua/config.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
> | 2026-11-04 | Delivered in full, except `Realized-by` and `Status: Done`: the `milestone` type does not yet declare `Realized-by` in `.urzua/config.yaml`, so the evidence cannot be recorded here. Unused dependencies removed (`anyhow`, `serde`/`serde_json` in `urzua-agdr`, and the unused `urzua-agdr`, `urzua-core` and `serde` edges `cargo machete` found); `cargo machete` and `cargo deny check` run in CI and `make ci`; a panic exits 2; a closed stdout no longer panics; `urzua new` reads date and author field names from `relation_fields.date`/`author`. **Why:** the Done-when. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Status` from `Realized-by computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-07 | Added the exercise the Done means asks for. **Why:** the independent review of `ADR-96` found the clause "exercised by this repository's own config" vacuous: `.urzua/config.yaml` declared no `relation_fields.date` or `author` for `new` to read. The `adr`, `rfc` and `spec` types now declare them (`Date`, `Author`), and `Realized-by` gains the config locator. They name the same fields as the defaults, so the non-default case (a type calling them `Created` and `Owner`) stays covered by the fixture only; no type here uses other names. No reopen: every other Done means item is evidenced. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
