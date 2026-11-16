---
Stable-Id: 01M7HW1FHMR5SK2PKYHGEE0BMN
Status: Planned
Phase: '1'
Track: release-process
Blocked-on: null
---
# 293 — A cancelled duplicate CI run does not block a merge

## What

Stop the required check from being left failed when two runs start for one commit and one is cancelled.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Opening a pull request and labelling it starts two runs; the first is cancelled, its `ci` job reports failure, and the merge is refused until that cancelled run is re-run by hand. It happened on nearly every records-only pull request.

## Done means

- Opening a pull request and adding a label within a minute leaves the required check green once the surviving run passes, with no manual re-run.
- The cause and the fix are recorded in the pipeline spec.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
