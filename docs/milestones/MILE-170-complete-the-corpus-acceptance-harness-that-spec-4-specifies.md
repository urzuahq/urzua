---
Stable-Id: 01M6K40D4E9SWM11EVM5723VGM
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-4
Blocked-on: —
---
# 170 — Complete the corpus acceptance harness that SPEC-4 specifies

## What

Finish the corpus acceptance harness `SPEC-4` specifies, so the foundation refactors (`MILE-171` to `MILE-176`) land under a safety net.

What exists: `rust/crates/urzua-cli/tests/acceptance.rs` is a custom runner (`harness = false`, so `cargo test --workspace` runs it) that materializes a corpus as a throwaway git repo, runs the real binary, and checks `expect_exit`, `expect_stdout_contains`, `expect_json`, `expect_findings` and `expect_no_findings`. It reads `manifest.yaml`, supports `known_gap` markers and the `URZUA_ACCEPTANCE_EXTRA` private-corpus hook. Four corpora are checked in (`corpus-a`, `corpus-b`, `corpus-c`, `corpus-zero`) with six cases.

What is missing: the cases `SPEC-4`'s tables name beyond those six; the check that each case fails against a build without its rule (mutation checking); the generative property; the "Nth record type" case; and a stated coverage of the rules in `ALL_RULES`, so a rule with no acceptance case is visible. Cases that reproduce documented bugs are not part of this milestone (that stays `MILE-101`, Phase 1).

## Why

The refactors in the foundation set rewrite rule wiring, `check::run`, option plumbing and the command preamble. Their only present safety net is `check_integration.rs` (over 3,000 lines of hand-built fixtures) and six acceptance cases, which is too thin to detect a behaviour change in a rewrite of the orchestrator. `ADR-55` says a rule must be able to fire and a test must assert it looked; the acceptance suite is where that is proven against a realistic corpus, not a case the author had in mind. `MILE-101` was Phase 0 until 2026-11-04 and its text said none of it existed; the runner then landed, so what is left splits cleanly into this harness work (a Phase 0 precondition for refactoring) and bug reproduction (Phase 1).

## Done means

Every case `SPEC-4` names for the foundation surface is executable under `expectations/`; each new case is observed failing without its rule; the runner reports which rules in `ALL_RULES` have no acceptance case; corpus zero (this repository) is exercised by at least one case per rule family; `SPEC-4` carries a revision row for what changed; and `cargo test --workspace` runs it with no new flags.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
