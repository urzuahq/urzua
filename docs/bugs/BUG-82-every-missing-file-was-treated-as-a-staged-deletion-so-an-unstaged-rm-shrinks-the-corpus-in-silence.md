---
Stable-Id: 01M2YPPQY0H0CZX2ECQY9CPME2
Status: Fixed
Found-in: "Round 9 of the 0.4.0 review, reproduced against a scratch repository"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::an_unstaged_deletion_of_a_tracked_record_stops_the_run"
---
# 82 — Every missing file was treated as a staged deletion so an unstaged rm shrinks the corpus in silence

## What was wrong

`BUG-76` made an unreadable tracked record stop the run, with one exception: a staged deletion is
legitimately absent from disk, so `ErrorKind::NotFound` was skipped.

It was skipped for *every* absence. `DiscoveredFiles` carries `paths` and `source` and nothing else,
so `load_records` had no way to tell a staged deletion from a file someone deleted and has not staged
-- and the comment justifying the skip asserted a distinction the type could not make.

Reproduced:

```text
$ urzua check                     -> ok, files_examined: 2, exit 0
$ rm docs/adr/ADR-2-b.md          # not staged
$ urzua check                     -> ok, files_examined: 1, exit 0
```

A tracked record deleted from the worktree, and the run still certifies the corpus clean. `BUG-76`
reopened by its own fix.

`DiscoveredFiles` now carries the staged deletions, and only those absences are skipped.

## Why nothing caught it

The test written with `BUG-76` covers invalid UTF-8, which is the read error the bug was reported
for. It never exercises the `NotFound` arm, so the exception added alongside the fix had no test at
all.

Worse than an untested branch: the comment stated the reasoning ("discovery unions `ls-files` with
the staged diff, so a staged deletion is legitimately not on disk") without anyone checking whether
the value being tested carried that information. It did not.

## References

- `BUG-76`, whose fix introduced this.
- `ADR-55`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
