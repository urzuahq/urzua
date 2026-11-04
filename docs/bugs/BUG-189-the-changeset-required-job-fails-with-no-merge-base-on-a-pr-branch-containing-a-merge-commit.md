---
Stable-Id: 01M6JYVPZ0C8AYB3SDQ987V57H
Status: Fixed
Found-in: 'a PR branch that merged main instead of rebasing, after main had moved (2026-11-04)'
Regression-test: 'No automated test exists for a workflow. Verified by reproduction: a repository with a PR branch holding a merge commit of main, then a further commit on main; a depth-1 clone plus `git fetch origin main --depth=1` makes `git diff --name-only origin/main...HEAD` exit 128 with "no merge base", and the same clone with `git fetch origin main` (no depth) lists the PR files. The two steps in .github/workflows/ci.yml now fetch without --depth.'
---
# 189 — The changeset required job fails with no merge base on a PR branch containing a merge commit

## What was wrong

The `changeset required` job checks out with `fetch-depth: 0`, then runs `git fetch origin <base> --depth=1` in two steps before a three-dot diff. The depth-1 fetch shortens the base branch's history to one commit, so once main had moved past the PR's fork point there was no common ancestor in the clone and `git diff base...HEAD` failed with "no merge base". A PR branch with a merge commit of main hit it; the only remedy was to rebase and force-push.

## Why nothing caught it

The job was exercised only on PRs whose base was still the fork point, where the one fetched commit happens to be the merge base. No test runs the workflow.

## References

Fixed in `.github/workflows/ci.yml`: both fetches drop `--depth=1`. The checkout is already full-depth, so the fetch only refreshes `origin/<base>` and stays fast.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and fixed. **Why:** agents were forced to rebase and force-push instead of merging main. | **substantive** |
