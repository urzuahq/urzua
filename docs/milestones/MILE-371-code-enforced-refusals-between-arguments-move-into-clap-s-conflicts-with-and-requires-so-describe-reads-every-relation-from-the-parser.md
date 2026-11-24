---
Stable-Id: 01M85VXPCSYBW4WES6G7KTYSCM
Status: Planned
Phase: '1'
Track: integrations
Depends-on: MILE-365
Blocked-on:
---
# 371 — Code-enforced refusals between arguments move into clap's conflicts_with and requires, so describe reads every relation from the parser

## What

Every refusal between two arguments that is a pure conflict or a pure requirement, and is refused
today in a command's own code (`check`'s and `fix`'s and `reverify`'s usage checks, `explain
--full`, `--as-of` with `--apply` in `main.rs`), is declared to clap as `conflicts_with` or
`requires` instead, and the hand-kept `CODE_RULES` table in `commands/describe.rs` is deleted, so
`describe` reads every relation from the parser.

## Why

`describe` declares relations refused in code from `CODE_RULES` (`MILE-359`, `ADR-108`), and a test
runs every declared pair and expects a refusal. The reverse is not caught: a refusal added in code
without a `CODE_RULES` entry fails no test, and `describe` then under-states the interface. The
published agent CLI contract's maintainers, answering review on pull request #647, said `describe`
states the interface, not how the parser is built; moving the refusals into clap makes the two the
same thing. It waits on `MILE-365`, which reworks how a refused command line is reported
(`error_kind` `usage`, an `error` without the parser's usage block): moving a refusal into clap
changes its message, and that rework decides what the message is.

## Done means

- Every pure conflict and requirement listed in `CODE_RULES` is declared with clap's `conflicts_with` or `requires` in `main.rs` or `WaiveArgs`, and `CODE_RULES` no longer exists.
- `urzua describe`'s `conflicts_with` and `requires` are unchanged from before the move, shown by a test that compares them.
- A refusal of one of these pairs still prints a `not-run` report, with `error_kind` `usage` once `MILE-365` has shipped.
- A "needs one of two" or "required unless" rule (`fix --apply` needs `--ids` or `--force`; `explain <path>`) stays in the argument's `means`, since the contract has no field for it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. Raised by review of pull request #647: a refusal added in code without a `CODE_RULES` entry fails no test. | **structural** |
