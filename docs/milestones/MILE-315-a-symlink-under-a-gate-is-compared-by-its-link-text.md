---
Stable-Id: 01M7K8KXPPA3MSZ7NNSGTHC1E3
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 315 — A symlink under a gate is compared by its link text

## What

Hash a symlink by the text of its link in every place the harness compares protected paths, so that retargeting one is a change.

## Why

On 2026-11-16 a review reproduced, with the released binary, a dangling symlink under a protected path being retargeted and committed with nothing reported: an unreadable target hashes to the same placeholder before and after, so the comparison sees no change. A pull that retargets it is silent the same way.

## Done means

- Retargeting a symlink under a protected path, dangling or not, is reported by the post-command step, shown by a test of each.
- The edit hook and the session baseline hash a symlink the same way, shown by a test that the three agree.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Found on 2026-11-16 by an independent review of a pull request; the maintainer said that day that friction and gaps met while using the tool are improvement points to file and fix. | **structural** |
