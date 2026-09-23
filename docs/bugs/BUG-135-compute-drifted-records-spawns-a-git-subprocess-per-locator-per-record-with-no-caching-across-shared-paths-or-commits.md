---
Stable-Id: 01M36S913RRNHGMYP5W7V0R3CW
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 24"
Regression-test: "not yet written -- Status: Open, performance-only, no correctness impact"
---
# 135 — compute_drifted_records spawns a git subprocess per locator per record with no caching across shared paths or commits

## What was wrong

`compute_drifted_records` (`discovery.rs`) calls `urzua_io::commit_for_line` once per record (a
blocking `git` subprocess), then for every `Realized-by` locator on that record calls
`last_commit_for_path` and `commit_strictly_before` (each another blocking `git` subprocess) -- with
no memoization across records or locators that share the same path or resolve to the same commit.

On a corpus with many embodied records, several of which cite the same file (a shared implementation
module, a shared test file), `check`'s wall-clock time scales as O(records × locators) git-process
spawns, each tens of milliseconds, with the same path's commit history looked up repeatedly.

## Why this is filed, not fixed here

Not a correctness bug -- `embodiment.consistency`'s findings are unaffected, only wall-clock time.
Fixing it means adding a cache layer (memoize `last_commit_for_path` per path, `commit_for_line` per
`(path, line)`) inside `compute_drifted_records`'s own loop, plumbed carefully against `urzua-io`'s
existing git-subprocess wrappers -- a real, contained change, but not a one-line fix, and this
repository's own corpus is not yet large enough for the cost to be visible in practice.

## References

- `compute_drifted_records`, `urzua_io::commit_for_line`/`last_commit_for_path`/
  `commit_strictly_before` -- the functions involved.
- `embodiment.consistency` (`ADR-18`/`ADR-32`) -- the rule this computation feeds.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified no caching exists across records/locators sharing a path or commit. Performance-only and not yet visible on this repository's own corpus size, left `Status: Open` for its own change. | **substantive** |
