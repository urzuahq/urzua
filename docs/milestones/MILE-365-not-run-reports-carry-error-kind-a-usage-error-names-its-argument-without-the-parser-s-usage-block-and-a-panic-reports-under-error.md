---
Stable-Id: 01M85PW8KKM4FP8E8JVE17TDDJ
Status: Planned
Phase: '1'
Track: integrations
Depends-on: MILE-359
Blocked-on: —
---
# 365 — Not-run reports carry error_kind, a usage error names its argument without the parser's usage block, and a panic reports under error

## What

`error_kind` on every `not-run` report (`usage`, `config`, `environment` or `internal`, as the contract defines them); a usage error's `error` is one sentence naming the argument that was wrong, with no clap `Usage:` block; and the panic report's text moves from the `panic` key to `error`, with `error_kind` `internal`.

## Why

Contract 1.1 (the published agent CLI contract's RFC-4) is taken whole: urzua prints `contract_version` `"1.1"` only when every piece below is delivered (`MILE-370`). Today `main.rs` passes `e.to_string()` of the argument parser to `refuse`, so the parser's prose and its `Usage:` block land in `error`; `CouldNotRun` (`urzua-core/src/report.rs`) has no kind; and `install_panic_hook` prints the text under `panic`. The contract treats `error` as text a reader never parses, and `error_kind` as the data. This is the smallest, independent piece and the one every later piece's tests lean on. The addition-only rule of `MILE-359` holds: no existing key is renamed or removed, except the `panic` key, which `SPEC-1` documents and the contract replaces with `error`; `SPEC-1` changes with a revision row.

## Done means

- A test runs urzua with a wrong argument, an unreadable configuration, a missing repository and a forced internal failure, and each `not-run` report carries `error_kind` `usage`, `config`, `environment` and `internal` respectively, checked against the report schema of the release candidate `MILE-369` vendors.
- A planted usage error's `error` contains no `Usage:` line and no line break; the test fails against the current `refuse(e.to_string())`.
- A test that forces a panic (the existing `panic_if_requested` seam) finds `error` and `error_kind` `internal` and no `panic` key, and exit code 2.
- A grep of `rust/crates/*/src` finds every construction of `CouldNotRun` setting a kind, so a new one without a kind does not compile or fails a test.
- `SPEC-1` (the panic report) and `SPEC-2` and `docs/reference/commands.md` describe `error_kind` and `error`, and each spec carries a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted: "I accept RFC, and yes allow option can be outside rule" (the contract's RFC-4, revision row of 2026-11-24). | **substantive** |
