---
Stable-Id: 01M2MP1F6VY4TNSH7J1QFMES1R
Status: Accepted
Embodiment: Implemented
Realized-by: code:.github/workflows/checks.yml, code:.github/workflows/ci.yml, code:.github/workflows/publish-release.yml
Date: 2026-09-16
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-45
---
# 47 — publish-release verifies the tree it ships, instead of asking whether ci ran

## Context

`ADR-45`'s amendment gave `publish-release.yml` a `verify-ci` job that queried the GitHub API for
the `ci` check-run's conclusion on the release PR's head sha, and refused to publish unless it read
`success`.

That gate cannot do what its name says. The release PR is opened by knope using `GITHUB_TOKEN`, and
GitHub holds workflow runs from such a PR at `action_required` until a human approves them. Until
someone clicks approve, the query returns `missing` — verified directly while shipping `v0.2.1`, on
commit `1abe6aa`. So `verify-ci` does not answer "did this tree pass"; it answers "did a human
approve a run." Merging before that approval publishes nothing and leaves `main` carrying a version
bump with no tag.

Two further facts make the original justification unsound. `ADR-45`'s amendment claimed the head sha
"definitively finished (it gated the merge)" — `main` has no branch protection, so nothing gates any
merge, and `#39` was `MERGEABLE` with `ci` never having run. And the head sha is not the released
tree: the tag lands on the squash commit created on `main` (`44939ca` for `v0.2.1`), a different
commit from the PR head (`1abe6aa`) and from the default checkout ref for the triggering event.

`RFC-30` proposed the other direction: act as a GitHub App so those events trigger workflows
normally. It would have worked.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Keep `verify-ci` as-is | No change | Gate is unsatisfiable without a human click; verifies a commit that is not the one being released |
| **Run the checks in `publish-release.yml`** | No cross-workflow dependency to be absent; verifies the exact released commit; no new identity or secret | Adds the check runtime to every publish; needs the job definition shared to avoid drift |
| A GitHub App token (`RFC-30`) | Removes the approval; events behave normally | A new external identity, a private key to hold and rotate, and it still only *infers* the released tree from the PR head |
| A classic PAT | Same trigger behaviour | Long-lived, person-tied, broader scope than this repo |

## Decision

In the context of a publish gate that could only ever report whether a human clicked approve, and
which checked a commit that is not the one being tagged, we decided: **`publish-release.yml` runs
the checks itself, against the exact commit it is about to release.**

- The build/test/self-host gate moves into `checks.yml`, a `workflow_call` reusable workflow.
  `ci.yml` and `publish-release.yml` both call it, so there is one definition rather than two that
  can drift.
- `publish-release.yml`'s `verify` job passes
  `ref: ${{ github.event.pull_request.merge_commit_sha }}` — for a squash merge, the commit
  created on `main`, which is the commit knope tags. Naming it beats inferring it.
- `verify-ci`'s API query is deleted, along with its `checks: read` permission.

This is the same move `BUG-28` made: a cross-workflow dependency that structurally could not fire was
removed rather than worked around.

**`RFC-30` is rejected.** Its three motivations are now covered without a new identity: the tag
trigger by `BUG-28` (and deliberately not depended on again — see Consequences), the stale label
payload by `BUG-34`, and the held `ci` run by this decision. `AGENTS.md`'s rule against speculative
capability applies to an App that would exist only to restore a trigger nothing needs.

## Reversibility

Cheap. Restore the `verify-ci` job from git history and delete `checks.yml`, inlining its job back
into `ci.yml`. No data format changes, no external state. Adopting `RFC-30` later remains open if a
future need — a required status check on `main`, say — makes the approval genuinely blocking rather
than merely tedious.

## Consequences

- **One manual step per release instead of two**, and the remaining one — merging the release PR — is
  the deliberate act `ADR-45` exists to create, not a defect.
- **The release PR's own `ci` run stays held** at `action_required` and the PR shows `UNSTABLE`. That
  is now cosmetic: nothing consumes its conclusion. Approving it remains available as an early
  signal. `ci` is deliberately **not** filtered off the release branch — `SPEC-20` already records
  why a filtered required check can deadlock a merge, and that trap stays shut.
- **The checks now run twice for a release** (once on the PR if approved, once here). Accepted: the
  second run is the one that verifies the released commit, and the cache makes it cheap.
- **`checks.yml` is called, never triggered.** It has no `on:` trigger other than `workflow_call`, so
  it cannot run on its own and has no independent check-run to mistake for a gate.
- **The check-run name changes** from `rust (fmt, clippy, test, build)` to a caller-prefixed form,
  because that is how reusable workflows report. Harmless today — `main` has no required checks — but
  it must be accounted for if branch protection is ever added.
- **This decision does not restore the tag-triggered build.** `BUG-28`'s fix stands on its own.
  Were a future token choice to make tag pushes trigger workflows again, splitting build/upload back
  out would re-couple binaries to a token decision, which is how `v0.2.0` shipped empty.

## References

- ADR-45 -- the two-step release flow; this replaces the `verify-ci` job its amendment introduced.
- BUG-28 -- the same move, one layer down: delete the dependency rather than work around it.
- BUG-34 -- the stale label payload, fixed alongside this.
- BUG-33 -- unrelated and still open; `prepare-release` going red on a `no-changeset` merge is
  knope's behaviour, not a trigger question.
- RFC-30 -- rejected here; the reasoning is kept because "why not just an App token" is the obvious
  question to ask later.
- SPEC-20 -- the CI/CD reference this changes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial decision. **Why:** shipping `v0.2.1` showed `verify-ci` could only ever report whether a human clicked approve, and that it checked a commit other than the one being tagged. Both were invisible until the flow ran end to end without a defect consuming the run. | **structural** |

## Amendment (2026-09-16): two Consequences corrected by the first real release

`v0.3.0` was the first release published under this decision, and it worked — `verify` ran the full
gate against `merge_commit_sha`, `release`/`build`/`upload` followed, three attested archives in two
minutes, no manual step. Two of the Consequences above describe it wrongly.

**"The release PR's own `ci` run stays held at `action_required`."** It does not run, correctly — but
it reports as **`failure` with zero jobs**, not as a pending hold. The distinction matters because a
red ✗ reads as "something broke" where a pending ⏸ reads as "waiting on you." It was mistaken for a
real failure within minutes of the release, by exactly the question the shape invites: *there was a
failure in the CI and it still got released.* Nothing was wrong — but the check's appearance argues
otherwise on every release, which is `BUG-33`'s "a gate nobody reads" arriving by another route.

**"The checks now run twice for a release (once on the PR if approved, once here)."** They ran
**once**. The "if approved" condition does not hold by default and, after this decision, there is no
reason for anyone to approve it. The doubled runtime this Consequence accepted as a cost is not being
paid.

`RFC-30` is reopened as `Draft` on the strength of the first point: an App-token-created PR would
execute its run and pass, which is the only mitigation that makes the check honest rather than merely
explained. Filtering it away is impossible (`pull_request.branches:` filters the base branch, and a
job-level `if:` never evaluates because the hold precedes job selection), and a synthetic passing
check would assert a verification that did not occur.

The decision itself is unchanged and is working. Only its description of the aftermath was wrong.
