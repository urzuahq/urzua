---
Stable-Id: 01M2MMFYSSGJWJ5X97Z0FQ5YHT
Status: Open
Found-in: 'shipping v0.2.1 -- the release PR''s approved `ci` run failed the changeset gate despite the PR carrying the `no-changeset` label, and only passed after a human removed and re-added the label by hand'
Regression-test: 'not yet written -- the assertion is that a release PR reaches a green `ci` with no human label handling, which is only observable on a real release run; RFC-30 would make it testable by removing the cause'
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

## The manual step this creates

Until fixed, every release needs a human to:

1. Approve the held `ci` run, and
2. Remove and re-add the `no-changeset` label, because step 1 approves the *stale* payload.

Step 2 is undocumented anywhere and is not discoverable from the failure — the job's message says to
add a label that is already present.

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
> | 2026-09-16 | Initial record, `Status: Open`. **Why:** found by running the release rather than reading the workflow -- `BUG-15`'s fix reads as complete in both the record and `SPEC-20`, and is inert in exactly the case both cite as its motivation. | **structural** |
