---
Stable-Id: 01M7GY90WHXBXBR6YZ3A793HE5
Status: Planned
Phase: '1'
Track: embodiment-model
Blocked-on: null
---
# 284 — Re-pins stop making every pair of code pull requests conflict

## What

Change how a re-pin is recorded or merged so that two pull requests that each re-pin the same records can both merge without hand-resolving every record.

## Why

On 2026-11-15 and 2026-11-16 one pull request re-pinned about 85 records and another about 100; each merge to the default branch made the other conflict in dozens of records, and the second had to be rebuilt from scratch three times. Whole-file pins on a few central files mean any change to them touches a large share of the corpus.

## Done means

- Two branches that each change a pinned file and re-pin the same record merge without a conflict in that record, or one documented command resolves all such conflicts and re-pins, shown by a test with two such branches.
- The number of records pinned to a whole central file falls, by narrower pins or back-pointers, and the count before and after is recorded.
- The contributor documentation says what to do when a branch's re-pins conflict.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Found while landing parallel pull requests on 2026-11-15 and 2026-11-16. | **structural** |
