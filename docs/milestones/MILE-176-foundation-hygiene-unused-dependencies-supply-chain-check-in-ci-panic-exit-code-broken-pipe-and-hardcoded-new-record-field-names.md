---
Stable-Id: 01M6K40T93YGAFC0QNNK4F53XM
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: —
Realized-by: code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, code:rust/deny.toml@sha256:5e3106a64b53e5e4de63ca3660d0d8ecca13f5c67747bb67a499fee4663f181b, code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436, code:rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf849d76d0f53912bc9c090f34f0666ef1a85b2a002b29747fe2ea14, test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_panic_exits_2_with_a_not_run_result@sha256:ed00fea0578930cb2a31aaa00e999fee926b483847dbbe029115ec6954d5e6ca, test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_closed_stdout_does_not_panic@sha256:6610198fac3875185be86ba8e8eac415d7e92e38a91a26320f924b0d9cca6cfb, code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_differently_named_date_and_author_fields_from_the_declaration@sha256:21637897a097da1db8c5af427984af8a6077f9e5a6a43a28934ff9c8672dea6d, code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e
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

- No unused dependency remains and CI fails on one. — evidence: code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d
- A vulnerability check runs in CI and `make ci`. — evidence: code:rust/deny.toml@sha256:5e3106a64b53e5e4de63ca3660d0d8ecca13f5c67747bb67a499fee4663f181b, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436
- A panic exits 2 and a test asserts it. — evidence: code:rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf849d76d0f53912bc9c090f34f0666ef1a85b2a002b29747fe2ea14, test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_panic_exits_2_with_a_not_run_result@sha256:ed00fea0578930cb2a31aaa00e999fee926b483847dbbe029115ec6954d5e6ca
- A closed stdout pipe exits without a panic and a test asserts it. — evidence: test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_closed_stdout_does_not_panic@sha256:6610198fac3875185be86ba8e8eac415d7e92e38a91a26320f924b0d9cca6cfb
- `urzua new` fills date and author fields from the type's declaration and a fixture with differently named fields gets them filled. — evidence: code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_differently_named_date_and_author_fields_from_the_declaration@sha256:21637897a097da1db8c5af427984af8a6077f9e5a6a43a28934ff9c8672dea6d
- The capability is exercised by this repository's own config. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
> | 2026-11-04 | Delivered in full, except `Realized-by` and `Status: Done`: the `milestone` type does not yet declare `Realized-by` in `.urzua/config.yaml`, so the evidence cannot be recorded here. Unused dependencies removed (`anyhow`, `serde`/`serde_json` in `urzua-agdr`, and the unused `urzua-agdr`, `urzua-core` and `serde` edges `cargo machete` found); `cargo machete` and `cargo deny check` run in CI and `make ci`; a panic exits 2; a closed stdout no longer panics; `urzua new` reads date and author field names from `relation_fields.date`/`author`. **Why:** the Done-when. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Status` from `Realized-by computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-07 | Added the exercise the Done means asks for. **Why:** the independent review of `ADR-96` found the clause "exercised by this repository's own config" vacuous: `.urzua/config.yaml` declared no `relation_fields.date` or `author` for `new` to read. The `adr`, `rfc` and `spec` types now declare them (`Date`, `Author`), and `Realized-by` gains the config locator. They name the same fields as the defaults, so the non-default case (a type calling them `Created` and `Owner`) stays covered by the fixture only; no type here uses other names. No reopen: every other Done means item is evidenced. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 6 locators pinned, 6 file-level, 0 scoped. Evidence: .github/workflows/checks.yml@sha256:4afa5fe0db59, rust/deny.toml@sha256:5e3106a64b53, Makefile@sha256:fb9fda4c8508, rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf84, rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5f, .urzua/config.yaml@sha256:151830e5926d Reason: Pinned the files as they are now after checking each against the record it evidences: acceptance.rs, the corpus manifests, the corpus-zero expectations and urzua-cli's Cargo.toml still make up the acceptance harness SPEC-4 specifies (MILE-170), table.rs and runner.rs still hold the one rule table and the runner that gates it (MILE-171), check.rs, gate.rs and discovery.rs still split the run into stages with derive_status (MILE-172), session.rs still holds the shared open preamble that the command modules call, audit.rs now through check (MILE-173), options.rs and config.rs still hold the typed option access (MILE-174), deny.toml, checks.yml, the Makefile, main.rs's panic hook and new_record.rs still carry the hygiene items (MILE-176), urzua-record.sh, AGENTS.md's suggestion rule, the PR template and config.yaml still back the agent commands (MILE-177), settings.json, urzua-claude-code.py and harness/run.rs still wire the record-edit hook (MILE-178), doctor.rs, the Makefile and .git-hooks/pre-commit still install and verify the git hooks (MILE-179), and the urzua-gate action scripts, checks.yml and ci.yml still install a pinned release, run the gate and report it (MILE-180). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
