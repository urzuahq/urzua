---
Stable-Id: 01M36S913RRNHGMYP5W7V0R3CW
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 24"
Regression-test: two_records_citing_the_same_locator_are_both_detected_as_drifted (rust/crates/urzua-cli/tests/check_integration.rs)
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

## Fix

`compute_drifted_records` now memoizes `last_commit_for_path` per locator path and
`commit_strictly_before` per `(reference_commit, locator_commit)` pair, both scoped to a single call's
loop rather than plumbed through `urzua-io`'s own wrappers -- each is a pure function of its cache key,
so a repeat lookup returns the prior result unconditionally. `commit_for_line` is unchanged: it reads
the citing record's own `Realized-by` line, which is per-record by construction and has nothing to
share across records.

A regression test with two records citing the same locator confirms both are still independently
detected as drifted -- the case a caching bug (a stale entry, or keying only on the locator path and
missing which record's reference commit it was compared against) would break first. Not an
`_observed_failing` test: the pre-fix code was already correct, only slow: this test only guards the
cache's correctness against a future edit, not a fix for a wrong verdict.

## References

- `compute_drifted_records`, `urzua_io::commit_for_line`/`last_commit_for_path`/
  `commit_strictly_before` -- the functions involved.
- `embodiment.consistency` (`ADR-18`/`ADR-32`) -- the rule this computation feeds.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified no caching exists across records/locators sharing a path or commit. Performance-only and not yet visible on this repository's own corpus size, left `Status: Open` for its own change. | **substantive** |
> | 2026-09-23 | Fixed. Memoized `last_commit_for_path` per path and `commit_strictly_before` per commit pair inside `compute_drifted_records`'s own loop. Regression test confirms two records citing the same locator are both still independently detected as drifted. | **substantive** |
