---
Stable-Id: 01M265K0MXRD14NFK6WWXHJNTZ
Status: Fixed
Found-in: 'hit manually at least three times this session (PRs #21, #23, and the knope-opened #24) before being understood as a general defect rather than a one-off flake: applying `no-changeset` to an already-open PR left the `changeset required` check reporting `fail`, requiring an unrelated empty commit each time to force a fresh evaluation'
Regression-test: 'no automated test -- GitHub Actions trigger-type behavior isn''t something this repo''s own test suite can exercise; verified by hand that `pull_request:` with no `types:` defaults to `[opened, synchronize, reopened]` (GitHub''s own documented default), which does not include `labeled`/`unlabeled`'
---
# 15 — ci.yml's changeset gate evaluates a stale pre-label event payload since labeled isn't a trigger type

## What was wrong

The `changeset` job's own condition is `!contains(github.event.pull_request.labels.*.name,
'no-changeset')` — correct in principle, but `ci.yml`'s `pull_request:` trigger declared no explicit
`types:`, which GitHub defaults to `[opened, synchronize, reopened]`. Applying the `no-changeset`
label to an already-open PR fires a real `labeled` webhook event, but since `ci.yml` never listens
for it, no new workflow run happens — the job's most recent run keeps reporting against whatever
label state existed at the PR's `opened` (or last `synchronize`) event, which is never the label
state a human (or a script) applies moments later. The check shows `fail` even though the actual,
current PR state has the opt-out label applied correctly.

## Why nothing caught it

The `no-changeset` label's own intended use (per `ADR-29`) is to apply it *when opening* a
fragment-less PR, not after — so the common case never exercises the gap. It surfaced repeatedly
this session specifically because every fragment-less PR filed was labeled as an afterthought,
post-creation, rather than in the same action that opened it (`gh pr create` doesn't take a label in
one step the way this session used it, and `ADR-45`'s own automated `CreatePullRequest` step has no
`labels` field at all — labeling is necessarily a separate, later step for both the manual and
automated case). Each time, it was worked around with an unrelated empty commit rather than
recognized as one general defect in `ci.yml`'s own trigger declaration until it hit the fully
automated `prepare-release` flow, where there was no human in the loop to notice and paper over it
with a manual nudge.

## References

- `.github/workflows/ci.yml` -- the `pull_request:` trigger, now declaring
  `types: [opened, synchronize, reopened, labeled, unlabeled]` explicitly.
- ADR-29 -- the `no-changeset` label's own decision record.
- PRs #21, #23, #24 -- each hit this and required a manual empty-commit workaround before the root
  cause was understood.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Found (after recurring silently across three separate PRs this session) and fixed same-day. `ci.yml`'s `pull_request:` trigger now explicitly lists `labeled`/`unlabeled` alongside the previously-implicit default three types, so a label change alone re-evaluates the gate without needing an unrelated commit. `Status: Fixed`. | **substantive** |
