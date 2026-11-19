---
Stable-Id: 01M7SY3C1FKNDGG1WFSA87QNHS
Status: Fixed
Found-in: reviewing the BUG-207 fix, which named the dry run for an unmatched --conflicts entry, and running urzua fix --conflicts --ids on a tree with nothing unmerged
Realized-by: test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#an_ids_entry_matching_nothing_in_a_tree_with_nothing_unmerged_is_a_failure@sha256:293a91f8d5f04a5aabcf3c9c41ad0711ce482267b59c47a34cd0f8e760cc310c
---
# 209 — urzua fix --conflicts --ids matching nothing, with nothing unmerged, reports nothing-conflicted and exits 0

## What was wrong

`urzua fix --conflicts --ids <entry>` in a tree git lists no unmerged file for reported `status: nothing-conflicted` and exited 0 even though the `--ids` entry matched nothing. The `failed` list named the entry, but the status and exit code said there was nothing to do, so a caller that read the exit code treated an entry that selected nothing as success. Every other `fix` mode reports such an entry as a failure and exits 1.

## Why nothing caught it

The status was decided from the unmerged set first, so an empty set returned before the failed entries were considered. The test for a tree with nothing unmerged ran without `--ids`, and the tests for an unmatched entry always had a conflicted record present.

## References

Fixed in `rust/crates/urzua-cli/src/commands/fix_conflicts.rs` (`status_of`): a failed entry makes the status `partial-failure`, with `urzua fix --conflicts` as `next_action`. With no `--ids` and nothing unmerged the status is still `nothing-conflicted` and the exit code 0. `SPEC-8` and `docs/reference/commands.md` state it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
