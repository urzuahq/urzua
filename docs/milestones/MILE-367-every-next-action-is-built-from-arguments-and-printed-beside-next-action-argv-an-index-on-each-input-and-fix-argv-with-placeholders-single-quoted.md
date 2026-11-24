---
Stable-Id: 01M85PWATEYZV2X9G3W58M1Q0N
Status: Planned
Phase: '1'
Track: integrations
Depends-on: MILE-359
Blocked-on: —
---
# 367 — Every next_action is built from arguments and printed beside next_action_argv, an index on each input and fix_argv, with placeholders single-quoted

## What

Every `next_action` is built from a program and its arguments, printed as a string joined and shell-quoted for POSIX sh, and as `next_action_argv` (null exactly when `next_action` is null); each entry of `next_action_inputs` carries an `index` of the element it replaces, that element being exactly its placeholder; every finding with a `fix` carries `fix_argv`; and a placeholder in the string is single-quoted in every report (`'<record>'`).

## Why

Contract 1.1 (the published agent CLI contract's RFC-4) is taken whole: urzua prints `contract_version` `"1.1"` only when every piece below is delivered (`MILE-370`). `NextAction` (`urzua-core/src/report.rs`) holds a `command: String`, and about 36 construction sites in `rust/crates/*/src` (`runs_as_printed`, `with_next_action` and the placeholder variants) build strings by hand with three quoting styles (single, double and bare). The contract says a reader of the array never parses quoting and the two forms must agree under POSIX splitting. A day's work was the estimate the contract's text records for building every action from arguments, and it is the largest piece, so it is its own milestone. `hooks.rs` reads `next_action` with `as_str()`, so the string stays.

## Done means

- `NextAction` holds the argument vector and derives the string from it, and no construction site passes a pre-joined command string, shown by a grep and by the type.
- A test runs each reporting command with a planted case that yields a `next_action` and checks, for every report, `next_action_argv` is present, not empty when not null, and equals the string split as a POSIX shell would (the contract's runner does the same: `MILE-369`).
- Every `next_action_inputs` entry has an `index`, and the element at it equals the entry's `placeholder`; the string carries it single-quoted. The test fails against the current bare and double-quoted forms.
- Every finding that carries a `fix` also carries `fix_argv`, shown by a test over the rules that print one (`doctor`, `hooks`, `fix`).
- The vendored report and finding schemas (`MILE-370`) accept the new keys and the reports validate against them.
- `SPEC-2` and the reference pages that show a `next_action` show the quoted form, each spec with a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted (relayed 2026-11-24: "accepted [the contract's] RFC-4 at 2026-11-24 00:02 UTC"). | **substantive** |
