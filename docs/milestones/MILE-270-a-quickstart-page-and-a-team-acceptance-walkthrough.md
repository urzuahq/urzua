---
Stable-Id: 01M7FW6XG81Z84JQCMSHNVGBCN
Status: Planned
Phase: '1'
Track: user-docs
Blocked-on: null
---
# 270 — A quickstart page and a team acceptance walkthrough

## What

Add one quickstart page (install, `init --types`, `new`, stage, `check`, accept by pull request) and a walkthrough of acceptance in a team where a maintainer approves a colleague's pull request, and correct the statements the walkthrough found false.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary and found no page that takes a new user from install to an accepted decision; acceptance by an approving maintainer is described only inside a long rule description; and eight statements in the reference pages and README were false or missing.

## Done means

- A quickstart page exists and a test runs its commands in an empty repository to a passing check.
- The CI page shows the configuration for acceptance by an approver, tested with an actors file.
- The reference pages agree with the binary on: the default file name `new` writes, whether greenfield creates type directories, what an ok status means with waived findings, and what `--actor` records.
- The harness page names the instruction file the adapter's agent reads.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
