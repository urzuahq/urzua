---
Stable-Id: 01M2WG81C9XDY7DQWF5NM7EWW4
Status: Fixed
Found-in: "Round 7 of the 0.4.0 review, reproduced against a scratch repository"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_scope_keeps_a_finding_about_a_file_git_does_not_track"
---
# 67 — A finding about a file git does not track is dropped by any path scope so check dot passes what check fails

## What was wrong

`BUG-60` made a path scope select which findings are reported. The filter keeps a finding when its
file is in scope or is the config, and `in_scope` is built from the *tracked* paths git reports.

`read_claim_files` does not consult git. It reads the declared paths straight off disk, so a claim
file that is not yet committed is never in `in_scope` and is not the config -- and its finding is
discarded.

| invocation | status | blocking | exit |
|---|---|---|---|
| `urzua check` | findings-present | true | 1 |
| `urzua check .` | ok | false | 0 |

Same corpus, same instant: `docs/bugs/BUG-1-x.md` is `Open` and an uncommitted claim file says
`Fixes BUG-1.`. Committing the claim file makes `check .` report it again, which isolates
tracked-ness as the discriminator.

`check .` is the natural pre-commit invocation, and an uncommitted fragment is the exact case
`claim.status-agreement` was written for. A gate that exits 0 on a corpus it should block is worse
than the fabricated findings `BUG-60` removed.

## Why nothing caught it

`in_scope` means *tracked and requested*. It was applied to findings, whose files come from two
different populations: records, which are discovered through git, and claim files, which are read
from disk. The proxy is exact for the first and wrong for the second, and the fix was tested only
against the first.

No test runs `check` and `check .` over the same corpus and compares them. The two invocations are
expected to agree, and nothing asserts that they do.

## References

- `BUG-60`, whose fix introduced this.
- `BUG-56` and `BUG-36`, on the rule whose findings are the ones dropped.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
