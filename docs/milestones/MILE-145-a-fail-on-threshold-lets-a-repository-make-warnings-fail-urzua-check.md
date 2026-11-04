---
Stable-Id: 01M6JCYYQ4SAV1Y7ST34B8YBCJ
Status: Done
Phase: '0'
Track: governance-process
Implements: SPEC-3
---
# 145 — A fail_on threshold lets a repository make warnings fail urzua check

## What

A top-level `fail_on: error | warning` key in `.urzua/config.yaml`, and a matching `urzua check
--fail-on error|warning` flag that overrides it, set the lowest unwaived finding severity that makes
`check` (and `audit`) exit 1. The default is `error`, so a repository that does not declare it is
unchanged. The JSON on stdout is identical under either value; only the exit code moves. This
repository sets `fail_on: warning`, so both CI steps (whole repo, and `--base` on a pull request)
fail on any warning.

## Why

`urzua check` on `main` reports no findings, but nothing stopped a warning from landing again: every
warning-level rule exits 0, and no flag or key raised the exit threshold. A zero-warning state that
only holds by attention regresses. The threshold is repository policy, so it is a config key like
every other behaviour (`ADR-53`), with a flag for a one-off stricter or laxer run.

## Done means

`fail_on: warning` makes a run with an unwaived warning exit 1 with unchanged stdout; a waived
warning does not; a clean run still exits 0; `--fail-on` overrides the key in both directions; any
other value is refused (a config error exits 2). Documented in `SPEC-3` and `SPEC-2`. Turning it on exposed `BUG-180`, a false warning every `--base`
run produced, fixed in the same change.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and done. **Why:** keep this repository at zero warnings by making them fail its own CI. | **substantive** |
