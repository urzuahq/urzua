---
Stable-Id: 01M85PW8KKM4FP8E8JVE17TDDJ
Status: Done
Phase: '1'
Track: integrations
Blocked-on: —
---
# 365 — Not-run reports carry error_kind, a usage error names its argument without the parser's usage block, and a panic reports under error

## What

`error_kind` on every `not-run` report (`usage`, `config`, `environment` or `internal`, as the contract defines them); a usage error's `error` is one sentence naming the argument that was wrong, with no clap `Usage:` block; and the panic report's text moves from the `panic` key to `error`, with `error_kind` `internal`.

## Why

Contract 1.1 (the published agent CLI contract's RFC-4) is taken whole: urzua prints `contract_version` `"1.1"` only when every piece below is delivered (`MILE-370`). Today `main.rs` passes `e.to_string()` of the argument parser to `refuse`, so the parser's prose and its `Usage:` block land in `error`; `CouldNotRun` (`urzua-core/src/report.rs`) has no kind; and `install_panic_hook` prints the text under `panic`. The contract treats `error` as text a reader never parses, and `error_kind` as the data. This is the smallest, independent piece and the one every later piece's tests lean on. The addition-only rule of `MILE-359` holds: no existing key is renamed or removed, except the `panic` key, which `SPEC-1` documents and the contract replaces with `error`; `SPEC-1` changes with a revision row.

## Done means

- A test runs urzua with a wrong argument, an unreadable configuration, a missing repository and a forced internal failure, and each `not-run` report carries `error_kind` `usage`, `config`, `environment` and `internal` respectively, checked against the report schema of the release candidate `MILE-369` vendors. — evidence: test:rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#every_not_run_report_carries_the_error_kind_of_its_cause@sha256:9de5a9334bef2f73c897ecc74b503bd65b3b15901127591387fdfab826da5cc3
- A planted usage error's `error` contains no `Usage:` line and no line break; the test fails against the current `refuse(e.to_string())`. — evidence: test:rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#a_usage_error_names_its_argument_in_one_sentence_without_the_parsers_usage_block@sha256:1c47f3d73f56fbaebe794f6dbad1a6b1cd97ed5f754e7b53d1efe4c5e8fb8cf6
- A test that forces a panic (the existing `panic_if_requested` seam) finds `error` and `error_kind` `internal` and no `panic` key, and exit code 2. — evidence: test:rust/crates/urzua-cli/tests/process_exit_integration.rs#a_panic_exits_2_with_a_not_run_result@sha256:cb2edc35923440c7ded7f8ce60eda411aa65b409b3c0e4b06d2ecc960ac74f3b
- A grep of `rust/crates/*/src` finds every construction of `CouldNotRun` setting a kind, so a new one without a kind does not compile or fails a test. — evidence: code:rust/crates/urzua-core/src/report.rs#CouldNotRun@sha256:8531ecd17448b58901883e51762ec4c58daa24c8034bcd27e31d270fb6b6b496
- `SPEC-1` (the panic report) and `SPEC-2` and `docs/reference/commands.md` describe `error_kind` and `error`, and each spec carries a revision row. — evidence: code:docs/reference/commands.md#L16-L22@sha256:25409c51aa16674e80b281a9ad4e29c16cf7f510380c9473b415b07ee5607aaf, code:docs/specs/SPEC-1-cli.md#L263-L274@sha256:71f93cb25e004777592c112ab87c2a95886aff401526cbd89c365bf1407c6180, code:docs/specs/SPEC-2-urzua-check.md#L301-L306@sha256:bf2f70ff5c43ff29f07e9e94d272c9746c6da0612a5a9dd3600f1781bc1200f0

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted: "I accept RFC, and yes allow option can be outside rule" (the contract's RFC-4, revision row of 2026-11-24). | **substantive** |
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 7 locators pinned, 0 file-level, 7 scoped. Evidence: rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#every_not_run_report_carries_the_error_kind_of_its_cause@sha256:9de5a9334bef, rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#a_usage_error_names_its_argument_in_one_sentence_without_the_parsers_usage_block@sha256:1c47f3d73f56, rust/crates/urzua-cli/tests/process_exit_integration.rs#a_panic_exits_2_with_a_not_run_result@sha256:cb2edc359234, rust/crates/urzua-core/src/report.rs#CouldNotRun@sha256:8531ecd17448, docs/reference/commands.md#L16-L22@sha256:25409c51aa16, docs/specs/SPEC-1-cli.md#L263-L274@sha256:71f93cb25e00, docs/specs/SPEC-2-urzua-check.md#L301-L306@sha256:bf2f70ff5c43 Reason: evidence for the Done-means items of MILE-365: the error_kind, usage-message and panic tests, the CouldNotRun constructor and the three documents **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-24 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (5 of 5 items resolved) computes to Implemented` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
> | 2026-11-24 | `Depends-on: MILE-359` removed: this milestone needs nothing `describe` delivers, and the dependency was a planning error in the plan that filed it; it is still part of contract 1.1, which `MILE-370` declares. **Why:** it capped the computed status at `InProgress` with every item delivered. | **structural** |
> | 2026-11-24 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (5 of 5 items resolved) computes to Implemented` -- stated `InProgress` disagreed with the computed value `Done`. | **structural** |
