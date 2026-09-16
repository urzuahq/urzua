---
Stable-Id: 01M2MMFYSSGJWJ5X97Z0FQ5YHT
Status: Fixed
Found-in: 'shipping v0.2.1 -- the release PR''s approved `ci` run failed the changeset gate despite the PR carrying the `no-changeset` label, and only passed after a human removed and re-added the label by hand'
Regression-test: 'the gate no longer reads the event payload at all, so staleness is unrepresentable rather than tested around -- the observable assertion is a release PR reaching green `ci` with no label handling, checkable on the next release'
Realized-by: code:.github/workflows/ci.yml
---
# 34 — The release PR's changeset gate can never pass on its own, because the label is applied with GITHUB_TOKEN

## What was wrong

`ci.yml`'s changeset job decides whether to run from the **event payload**:

```yaml
if: >-
  github.event_name == 'pull_request' &&
  !contains(github.event.pull_request.labels.*.name, 'no-changeset')
```

`knope.toml` opens the release PR and then labels it, in that order — `CreatePullRequest` has no
labels field, so the label is a separate `gh pr edit release --add-label no-changeset` step. The
`opened` event therefore carries a payload with no labels, and the job runs when it should skip. It
then fails, correctly by its own logic: the release PR consumes fragments and adds none.

`BUG-15` already fixed this class, by adding `labeled`/`unlabeled` to the trigger types so that
applying a label fires a fresh run with an updated payload. `SPEC-20` records it as *"always the case
for `prepare-release.yml`'s own automated PR"*.

**That fix is inert here.** knope applies the label using `GITHUB_TOKEN`, and GitHub does not trigger
workflows from `GITHUB_TOKEN` events. The corrective run `BUG-15` depends on never fires.

Re-running does not help either: GitHub replays a run with its original event payload, so a re-run
of the `opened` run sees no label again.

Observed while shipping `v0.2.1`. The gate only passed after a human removed and re-added the label,
which fired a genuine `labeled` event — and that run needed **no approval at all**, since the event
came from a real user.

## Why nothing caught it

`v0.2.0` was the flow's first end-to-end run and it was consumed by `BUG-27` and `BUG-28`; this gate
was never reached in a state where it could be observed. `BUG-15`'s own fix was verified by applying
a label as a human to a human-created PR, where it works correctly. Nothing exercised the one case
the fix names in its own record as the reason it exists.

This is the third symptom of one cause. `BUG-28`: a tag pushed with `GITHUB_TOKEN` triggers nothing,
silently. The release PR's own `ci`: created but held at `action_required`. This: a label applied
with `GITHUB_TOKEN` triggers nothing, so a fix that depends on that trigger does nothing.

## The fix

Stop reading the label from the event payload. The job now queries the PR's current labels at run
time:

```yaml
if: github.event_name == 'pull_request'     # job always starts
...
- name: check for the no-changeset opt-out
  run: gh pr view "$NUMBER" --json labels --jq '.labels[].name' | grep -qx 'no-changeset'
```

A payload snapshot can be stale; the PR's own state cannot be. This removes the staleness rather
than working around it, so it holds for a re-run too — a re-run replays the original payload, which
the job no longer consults.

`BUG-15`'s `labeled`/`unlabeled` trigger types stay, and remain load-bearing. The live read fixes a
*stale* payload — including on a re-run, which replays the original one — but not a label applied
*after* a run has started. That run correctly sees no label and fails, and only a `labeled` event
starts the correcting run.

Observed on `#43`: the `opened` run began at 09:11:02, the label landed at 09:12:29, and the
`labeled` run at 09:12:31 passed. The first failure was the gate working, not the defect recurring.
For the release PR specifically the ordering cannot bite, because the run is held at
`action_required` until a human approves it and knope's labelling step has long since finished.

Two consequences worth naming:

- The job now **runs and passes** where it previously **skipped**. That is the better shape under
  `ADR-29`'s "never a silent skip" rule: the log states that the opt-out was found, instead of the
  check simply not appearing.
- It needs `pull-requests: read`, declared at the job rather than widening the workflow default.

**The approval gate is not fixed by this** and is a separate cause: a `GITHUB_TOKEN`-created PR still
has its run held at `action_required`. That is one remaining manual step per release, addressed by
`RFC-30`. Before this fix it was two, and the second was undiscoverable — the failing job asked for
a label that was already present.

## References

- BUG-15 -- the fix this defeats; its `labeled`/`unlabeled` trigger types are correct and unreachable
  for this PR.
- BUG-28 -- the same cause, silent rather than noisy.
- RFC-30 -- the proposed fix: an App token, whose events do trigger workflows.
- ADR-45, SPEC-20 -- the flow and its spec; `SPEC-20` claims `BUG-15` covers this case.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change. **Why:** the workaround that unblocked `v0.2.1` -- removing and re-adding the label by hand -- is not something a release should require, and reading live state instead of a snapshot removes the failure mode rather than routing around it. The held-run approval is a different cause and stays open under `RFC-30`. | **substantive** |
> | 2026-09-16 | Initial record, `Status: Open`. **Why:** found by running the release rather than reading the workflow -- `BUG-15`'s fix reads as complete in both the record and `SPEC-20`, and is inert in exactly the case both cite as its motivation. | **structural** |
