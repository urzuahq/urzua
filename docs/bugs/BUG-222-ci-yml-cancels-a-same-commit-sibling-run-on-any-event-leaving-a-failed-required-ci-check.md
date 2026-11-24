---
Stable-Id: 01M85B7WGWK28WSPW2T9XQ9JA2
Status: Fixed
Found-in: 'reviewing the release flow: a release pull request merge was refused by the ruleset while the other run on the same commit was green'
Realized-by: code:.github/workflows/ci.yml@sha256:d7e1bf9840e867dfa7246e3cd261ddf2e4cd1914cb239d63b8f380ac4379d6de
---
# 222 — ci.yml cancels a same-commit sibling run on any event, leaving a failed required ci check

## What was wrong

`.github/workflows/ci.yml` starts on `pull_request` types `opened, synchronize, reopened, labeled, unlabeled,
edited` (line 14) and sets `cancel-in-progress: ${{ github.event_name != 'push' }}` (line 23) in a group
keyed on the ref (line 21). Two events for one commit (for example `opened` and then `labeled`, which the
release pull request gets from `gh pr edit release --add-label no-changeset` in `knope.toml`) start two runs
in one group, and the later run cancels the earlier. The cancelled run reports a failed required `ci`
check on that commit. The repository ruleset then refused the release pull request's merge while the other
run on the same commit was green; it took re-running the cancelled run to clear it. The repository's run
history shows two `pull_request` runs of `ci` for one release-branch commit started in the same minute.

The cancellation is wanted only when a new commit supersedes an old one. For a second event on the same commit
it destroys a run that was correct, and the label-correction design in the comment on lines 7-11 depends on a
`labeled` run existing beside the earlier one.

A gate that skips when cancelled is not a fix: GitHub counts a skipped required check as passing, so the
gate would then pass on a commit nobody checked.

## Why nothing caught it

The group and the cancel expression were written for superseding commits, and the `labeled` and `unlabeled`
types were added later (`BUG-15`) without asking what a second event for the same commit does to the first.
No test can exercise it, since it is GitHub's scheduler and not a script in this repository.

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

## Observation (2026-11-24): a third same-commit event may replace the pending run

A sibling repository was observed once with the case the caveat above leaves unchecked: a third event for one commit replaced the run waiting in the group. The replaced run showed `cancelled` with zero jobs and posted no `ci` check, and the head commit carried one green `ci`. This is one observation, not proof: it was not reproduced, and whether the missing check blocks a required-check ruleset is not shown here. It bears on the proposed edit, whose `cancel-in-progress` is false for every event but a new commit: with that edit, a third event can still replace the waiting run, so the group key that includes the event action (the fallback the caveat names) is worth deciding before the edit is applied.

## References

- `.github/workflows/ci.yml` (lines 12-23, 184-199), `knope.toml` (the `gh pr edit release --add-label no-changeset` step), `docs/specs/SPEC-20-ci-cd-pipeline.md`, `BUG-223`, `ADR-95`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Added the section `Observation (2026-11-24)`: one observation from a sibling repository of a third same-commit event replacing the pending run (cancelled, zero jobs, no `ci` check posted, head commit carrying one green `ci`). Nothing else changed and `Status` stays `Open`. **Why:** the earlier caveat said whether a replaced waiting run leaves a failed required `ci` was unverified; this is a first data point, and a single one. | **structural** |
> | 2026-11-24 | `Status` set to Fixed: `ci.yml` drops `edited` and sets `cancel-in-progress` true only for a `synchronize` event on a pull request, so a second event for one commit no longer cancels a correct run; `Realized-by` pins the workflow. The third-same-commit-event caveat (a replaced waiting run) remains unverified and is not resolved by this change. **Why:** the fix landed in the pull request that carries this row. | **substantive** |
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: .github/workflows/ci.yml@sha256:d7e1bf9840e8 Reason: ci.yml: the pull_request types drop edited, workflow_dispatch is removed, and cancel-in-progress is true only for a pull_request synchronize event; this is the fix the record describes **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-24 | Pointed to `BUG-226`, which records the open third-event case from the caveat and the observation above (a replaced waiting run; whether it leaves a missing or failed required `ci` is unverified) and the remedies it names. Nothing else changed. **Why:** the observation said the fallback group key was worth deciding before the edit was applied; the edit is applied and nothing recorded that question. | **structural** |
