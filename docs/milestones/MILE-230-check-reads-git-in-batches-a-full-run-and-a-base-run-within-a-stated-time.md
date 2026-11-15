---
Stable-Id: 01M7FVVNMW7B6SAD2N4EY1QHHF
Status: Planned
Phase: '1'
Track: scale-and-analytics
Blocked-on: null
---
# 230 — check reads git in batches: a full run and a base run within a stated time

## What

Replace the one-subprocess-per-record git reads with batched reads, and set a time budget for `check`, `check --base` and the editor-hook path that CI enforces.

## Why

the audit of 2026-11-15 measured 10.5 s for `check`, 21.4 s for `check --base` (the pre-commit hook) and 4.4 s for one record on 594 records, with one `git show` per changed record and one `git log -1` per locator path although a batch reader already exists.

## Done means

- Base-revision reads go through one batched git process per run, shown by a test that counts spawned processes.
- A benchmark in CI fails when `check`, `check --base` or a single-record run on this repository exceeds its stated budget.
- The stated budgets and the measured times are recorded in the scale spec or reference page.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
