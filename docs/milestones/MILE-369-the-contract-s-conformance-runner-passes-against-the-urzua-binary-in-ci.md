---
Stable-Id: 01M85PWD56FXDNSZBVFQ9K0J5Z
Status: Planned
Phase: '1'
Track: integrations
Depends-on: COND-0007, MILE-359, MILE-365, MILE-366, MILE-367, MILE-368
Blocked-on: —
---
# 369 — The contract's conformance runner passes against the urzua binary in CI

## What

This repository's CI runs the contract's conformance runner (`scripts/conform.py`, Python) against the built `urzua` binary, at the release candidate the contract tags with the runner (`COND-0007`) and that this milestone vendors (`VERSION`, schemas, `SHA256SUMS`), in a directory where `describe` and every command it marks `safe` run, and the job is part of the required gate.

## Why

Contract 1.1 (the published agent CLI contract's RFC-4) is taken whole: urzua prints `contract_version` `"1.1"` only when every piece below is delivered (`MILE-370`). Passing the runner in CI is a condition of 1.1 (RFC-4 section 9). The runner is in no contract tag yet (`COND-0007`), and the contract's section 10 has its 1.1 tag wait for both tools to pass it against a release candidate, so this milestone vendors and runs against that candidate and `MILE-370` vendors the final tag. The runner checks `describe`, empty stderr, exit codes against `status`, the argument forms and `--version`. It does not run hooks (section 5.1 is tested by each tool, here in `MILE-368`). The edit to `.github/workflows` is the maintainer's, because it is a protected path; the runner's pinned source, its invocation and the job it adds are written in the PR that ships it.

## Done means

- A workflow job runs the runner against `rust/target/release/urzua` at the vendored release candidate and fails the `ci` gate when the runner fails (the maintainer's edit; the exact job is in the PR).
- The runner passes locally against a release build, and a planted violation (a command that writes to stderr, found in `MILE-366`'s test) makes it fail, shown in the PR description.
- `SPEC-20` and `docs/reference/ci.md` name the job, each spec with a revision row, and `make ci` runs the same command so a contributor can run it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted: "I accept RFC, and yes allow option can be outside rule" (the contract's RFC-4, revision row of 2026-11-24). | **substantive** |
