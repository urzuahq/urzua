---
Stable-Id: 01M37DNTD6SQV4CB32PKQ6QQXV
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 27"
Regression-test: "a_staged_rename_is_discovered_under_its_new_name_only_observed_failing, urzua-io/src/lib.rs"
---
# 150 — discover_tracked_files spawns a redundant third git subprocess for staged deletions

## What was wrong

`discover_tracked_files` spawned three separate `git` subprocesses per invocation: `ls-files`,
`diff --cached --name-only` (staged), and `diff --cached --diff-filter=D --name-only` (deleted). The
third call's output is derivable from a single `--name-status` call's own status codes, which the
second call didn't request (`--name-only` carries no status). Every invocation of
`check`/`audit`/`fix`/`graph`/`explain`/`migrate` paid for an extra subprocess spawn entirely redundant
with information a differently-flagged version of the second call would already carry.

## Why nothing caught it

Nothing measured subprocess count directly, and three git calls under 10ms each on this repository's
own corpus size never showed up as a cost worth questioning.

## Fix

Merged the staged/deleted calls into one `git diff --name-status --cached -z`, parsed by status code.
Not a naive one-status-one-path parse: a rename or copy (`R`/`C`) reports *two* path fields per record
(`<status>\0<old-path>\0<new-path>\0`), not the one every other status carries, so a naive parser would
misalign on exactly that input. Handled explicitly, and verified against real git output (not assumed
from documentation) with a planted-violation test using an actual staged rename. A rename's old name is
deliberately still not counted as a staged deletion, matching the original two-call version's own
behavior: `--diff-filter=D` never matched a rename either, since git classifies it `R`, not `D`.

## References

- The regression test itself is the verification: a naive two-fields-per-record parser was confirmed to
  fail it (misaligning on the rename's extra field) before the fix, and pass after.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-24 | Filed and fixed in one pass, found by a full-release code review. **Why:** a redundant subprocess spawn on every invocation, closed by parsing one call's richer output instead of two calls' overlapping ones. The rename/copy two-path-field case was the real risk in this merge and is explicitly handled and tested. | **substantive** |
