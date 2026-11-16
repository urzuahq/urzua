---
Stable-Id: 01M7K3JEY6216HKDJ60VGHZC8R
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 311 — A hook that refuses prints the findings, not the whole report

## What

Have `urzua hooks run` print, on refusal, only the blocking findings and the next action, with the full report available by a flag or a file path it names.

## Why

On 2026-11-16, in an adopting repository, a refused commit printed the full JSON report of the check into the terminal: several hundred lines in which the one blocking finding was hard to find, for an agent paying for every line it reads. The quiet-by-default rule already applied to the agent harness hooks does not yet cover the git hooks.

## Done means

- On refusal `urzua hooks run` prints JSON holding only the blocking findings, their count, and the `next_action`, shown by a test that the output for one blocking finding among many records stays under a declared size.
- On success it prints nothing unless the configuration asks for output.
- The full report is reachable from the refusal: a flag prints it, or the output names the file it was written to.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Met on 2026-11-16 while re-pinning records in an adopting repository; the maintainer said that day "when you're seeing issues with like the use of the tool and how we adopt the tool, these are improvement points for us". | **structural** |
