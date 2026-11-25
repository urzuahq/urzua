---
Stable-Id: 01M85B7YFF7ATC30EKAN8RYKGW
Status: Fixed
Found-in: 'reviewing ci.yml: workflow_dispatch is a trigger, the changeset job runs on pull_request only and the gate allows it to skip on every other event'
Realized-by: code:.github/workflows/ci.yml@sha256:d7e1bf9840e867dfa7246e3cd261ddf2e4cd1914cb239d63b8f380ac4379d6de
---
# 223 — A manual ci run on a pull request branch lands in its own concurrency group and can leave a green ci over a failed changeset gate

## What was wrong

`.github/workflows/ci.yml` lists `workflow_dispatch:` (line 18). The `changeset` job runs only when
`github.event_name == 'pull_request'` (line 42) and the `ci` gate sets `ALLOW_SKIPPED` to `changeset` for
every other event (line 198), so a manual run skips the changeset gate and passes.

A manual run on a pull request's branch has `github.ref` `refs/heads/<branch>`, while the pull request's own
run has `refs/pull/<n>/merge`. The two land in different concurrency groups (line 21), so they run side by
side and neither cancels the other. Both attach a check named `ci` to the same head commit. If the manual run
finishes last, the commit carries a green `ci` on top of a pull request run whose `changeset` gate failed, and
the required check no longer says the gate failed. The only use of the trigger found in the repository is
`docs/specs/SPEC-20-ci-cd-pipeline.md` line 28, which lists it; nothing documents when to dispatch the workflow.

## Why nothing caught it

`ALLOW_SKIPPED` was written to let `changeset` skip where it does not exist (`push`, `merge_group`), and
`workflow_dispatch` was listed without asking whether a run on a branch with an open pull request is such a
place. It is not: the pull request exists and the gate that was skipped is the one it needs.

## The workflow edit (the maintainer's to apply; `.github/workflows` is a protected path)

In `.github/workflows/ci.yml`, replace the `on:` and `concurrency:` blocks (lines 12-23) with:

```yaml
on:
  pull_request:
    types: [opened, synchronize, reopened, labeled, unlabeled]
  merge_group:
  push:
    branches: [main]

concurrency:
  group: ci-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' && github.event.action == 'synchronize' }}
```

Its cost: a label added while a run is in progress now starts a second full run instead of cancelling the
first, and changing a pull request's base branch starts no run, because `edited` is gone and `changeset`
diffs against `github.event.pull_request.base.ref`. Also change alongside it, in the same PR:

- The comment above `on:` (lines 4-11) says the types are the default set plus `labeled`/`unlabeled`; it must
  say `edited` is not listed and why, and that there is no `workflow_dispatch`. Its claim that the label-then-run
  correction relies on a `labeled` event still reads correctly.
- The comment above `cancel-in-progress` (line 22) must say that only a new commit on a pull request cancels a
  run, so a second event for the same commit never does.
- `ALLOW_SKIPPED` (line 198) needs no change: with the dispatch trigger gone, `github.event_name != 'pull_request'`
  is true only for `push` and `merge_group`, where `changeset` does not exist. The comment above the `ci` job
  (lines 180-183) still reads correctly.
- `docs/specs/SPEC-20-ci-cd-pipeline.md`: lines 27-30 list `edited` and `workflow_dispatch` and describe the
  concurrency group, and lines 104-105 say a run is cancelled "on a PR title or body edit (`edited`)". Both
  change with a `Version` bump and a revision-log row, and the `Realized-by` pin on `ci.yml` is re-pinned
  after reading the diff (`AGENTS.md`, "Re-pinning evidence").
- A caveat not checked on GitHub: with `cancel-in-progress` false, a third run queued while one runs and one
  waits replaces the waiting run, which is cancelled before it starts. Whether that leaves a failed required
  `ci` is unverified; if it does, a group key that includes the event action for non-`synchronize` runs avoids it.

## References

- `.github/workflows/ci.yml` (lines 18, 21, 42, 198), `docs/specs/SPEC-20-ci-cd-pipeline.md` (line 28), `BUG-222`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | `Status` set to Fixed: `ci.yml` drops `workflow_dispatch`, so no manual run can post a `ci` over a pull request's changeset gate; `Realized-by` pins the workflow. **Why:** the fix landed in the pull request that carries this row. | **substantive** |
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: .github/workflows/ci.yml@sha256:d7e1bf9840e8 Reason: ci.yml: the pull_request types drop edited, workflow_dispatch is removed, and cancel-in-progress is true only for a pull_request synchronize event; this is the fix the record describes **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
