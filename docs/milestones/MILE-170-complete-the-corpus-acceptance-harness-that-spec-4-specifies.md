---
Stable-Id: 01M6K40D4E9SWM11EVM5723VGM
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-4
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/acceptance.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-g/manifest.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-h/manifest.yaml, test:rust/crates/urzua-cli/tests/acceptance_coverage_gate.rs#a_new_rule_without_a_failing_case_fails_the_gate@sha256:c96ec8f1e59d58a130634a0b22c2c395c99542ac944d09b090c0f55275a77dba, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/manifest.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-header-rules-run.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-embodiment-rules-run.yaml, spec:docs/specs/SPEC-4-the-corpus-acceptance-suite.md, code:rust/crates/urzua-cli/Cargo.toml
---
# 170 — Complete the corpus acceptance harness that SPEC-4 specifies

## What

Finish the corpus acceptance harness `SPEC-4` specifies, so the foundation refactors (`MILE-171` to `MILE-176`) land under a safety net.

What exists: `rust/crates/urzua-cli/tests/acceptance.rs` is a custom runner (`harness = false`, so `cargo test --workspace` runs it) that materializes a corpus as a throwaway git repo, runs the real binary, and checks `expect_exit`, `expect_stdout_contains`, `expect_json`, `expect_findings` and `expect_no_findings`. It reads `manifest.yaml`, supports `known_gap` markers and the `URZUA_ACCEPTANCE_EXTRA` private-corpus hook. Four corpora are checked in (`corpus-a`, `corpus-b`, `corpus-c`, `corpus-zero`) with six cases.

What is missing: the cases `SPEC-4`'s tables name beyond those six; the check that each case fails against a build without its rule (mutation checking); the generative property; the "Nth record type" case; and a stated coverage of the rules in `ALL_RULES`, so a rule with no acceptance case is visible. Cases that reproduce documented bugs are not part of this milestone (that stays `MILE-101`, Phase 1).

## Why

The refactors in the foundation set rewrite rule wiring, `check::run`, option plumbing and the command preamble. Their only present safety net is `check_integration.rs` (over 3,000 lines of hand-built fixtures) and six acceptance cases, which is too thin to detect a behaviour change in a rewrite of the orchestrator. `ADR-55` says a rule must be able to fire and a test must assert it looked; the acceptance suite is where that is proven against a realistic corpus, not a case the author had in mind. `MILE-101` was Phase 0 until 2026-11-04 and its text said none of it existed; the runner then landed, so what is left splits cleanly into this harness work (a Phase 0 precondition for refactoring) and bug reproduction (Phase 1).

## Done means

- Every case `SPEC-4` names for the foundation surface is executable under `expectations/`. — evidence: test:rust/crates/urzua-cli/tests/acceptance.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-g/manifest.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-h/manifest.yaml
- Each new case is observed failing without its rule. — evidence: test:rust/crates/urzua-cli/tests/acceptance.rs
- The runner reports which rules in `ALL_RULES` have no acceptance case. — evidence: test:rust/crates/urzua-cli/tests/acceptance.rs, test:rust/crates/urzua-cli/tests/acceptance_coverage_gate.rs#a_new_rule_without_a_failing_case_fails_the_gate@sha256:c96ec8f1e59d58a130634a0b22c2c395c99542ac944d09b090c0f55275a77dba
- Corpus zero (this repository) is exercised by at least one case per rule family. — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/manifest.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-header-rules-run.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-embodiment-rules-run.yaml
- `SPEC-4` carries a revision row for what changed. — evidence: spec:docs/specs/SPEC-4-the-corpus-acceptance-suite.md
- `cargo test --workspace` runs it with no new flags. — evidence: code:rust/crates/urzua-cli/Cargo.toml, test:rust/crates/urzua-cli/tests/acceptance.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
> | 2026-11-04 | `Status` `Planned` -> `InProgress`; no `Realized-by`. **Why:** the harness machinery is built, but Done means every `SPEC-4` case for the foundation surface is executable and this change does not reach that: the "silent zeros" git-history cases, the drift-while-exiting-0 case and several table rows have no case, and four rules (the report names them) have none either. Delivered: mutation proof of every case, the uncovered-rules report, the "Nth record type" case, the generative property in `acceptance_property.rs`, `corpus-d` to `corpus-f`, a corpus-zero case per rule family, and the `SPEC-4` 0.5 revision. `Realized-by` waits for the PR that finishes the remaining cases. | **substantive** |
> | 2026-11-04 | `Status` `InProgress` -> `Done`; `Realized-by` added (the runner, the property, and the manifests of the two corpora this change adds; corpora `d` to `f` are covered by the same runner). **Why:** the remaining Done-when items are met: the runner can make further commits, pass `--base` and run in a shallow clone, and `corpus-g` (git history: a shallow clone is a notice, a value-only edit and a renamed record are drift, drift blocks the exit, a neighbouring file is not drift, a mention does not define a back-pointer, a shared locator is a promotion candidate) and `corpus-h` (the two `diff.*` rules, the published-path boundary, `citation.issue-exists` against a fixture snapshot, `status.delivery-agreement`, and the permanent-ceiling case) are executable. The coverage report now shows 53 of 53 rules proven, none uncovered. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
