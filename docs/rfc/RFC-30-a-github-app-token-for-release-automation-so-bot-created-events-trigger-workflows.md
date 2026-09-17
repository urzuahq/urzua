---
Stable-Id: 01M2MMFZ5XEV4R5BX3DFGZ5PZM
Status: Draft
Date: 2026-09-16
Author: beauwilliams
---
# 30 — A GitHub App token for release automation, so bot-created events trigger workflows

> **Rejected (2026-09-16) by `ADR-47`, then reopened the same day.** The rejection reasoning stands
> and is kept below: `ADR-47` removed the *need* for the release PR's own `ci` run, so an App token
> was no longer buying a capability. What the first real release under `ADR-47` showed is that it was
> still buying something — see "Residual cost" immediately below.

## Summary

Every automated step in the release flow acts as `GITHUB_TOKEN`, and GitHub deliberately does not
trigger workflows from `GITHUB_TOKEN` events. Three separate defects follow from that one fact.
Propose minting a short-lived installation token from a GitHub App for the push, PR-creation, and
labelling steps, so those events behave like a real actor's.

## Residual cost that reopened this (2026-09-16)

`v0.3.0` was the first release under `ADR-47`. It published correctly: three attested archives, two
minutes, no manual step. And the release PR's own `ci` check was **red**, as it is on every release
PR, because GitHub does not execute a workflow run on a PR that `GITHUB_TOKEN` created.

Every `ci` run on the `release` branch, one day's worth:

| run | conclusion | triggered by |
|---|---|---|
| 35071226718 | failure | bot |
| 35072271054 | cancelled | a human, removing a label |
| **35072279374** | **success** | **a human, re-adding it** |
| 35077097004 | failure | bot |
| 35163583796 | failure | bot |

The only green run was triggered by a person. The state is reported as `action_required` on the run
object and as `failure` in listings — the same held run, described two ways.

`ADR-47` makes this cosmetic: nothing reads that check, and `publish-release.yml`'s own `verify` job
checks the exact commit being tagged. But cosmetic is not free. It was mistaken for a real failure
within minutes of the first release — *"there was a failure in the CI and it still got released"* —
and a permanently-red check on a routine event is what trains a reader to stop reading red checks.
That is `BUG-33`'s lesson, arriving by a different route on the same day it was fixed elsewhere.

**Neither available mitigation removes the red mark.** `pull_request.branches:` filters by *base*
branch, so the release PR cannot be excluded at the trigger; a job-level `if:` never evaluates,
because the hold precedes job selection; approving it each release restores the manual step
`ADR-47` deleted; and posting a synthetic passing check would be asserting a verification that did
not happen, in a repository whose entire subject is not doing that.

An App-token-created PR triggers workflows normally, so the run would execute and pass. That is the
only option that makes the check honest rather than explained.

**What would make this urgent rather than annoying:** branch protection on `main` requiring `ci`.
`main` has none today, which is why a red check blocks nothing. Add it, and the release PR becomes
unmergeable without either this proposal or a per-release manual approval. The two decisions should
be made together.

## Motivation

The recursion guard is correct and not something to argue with: without it, a workflow that pushes
could re-trigger itself indefinitely. The problem is that this flow depends on exactly the triggers
it suppresses. Three observed instances, all from real runs:

| Event | What GitHub did | Cost |
|---|---|---|
| `knope release` pushes the `v*` tag | No run created | `v0.2.0` published with zero binaries and no error (`BUG-28`) |
| knope opens the release PR | Run created, held at `action_required` | A human must approve before `verify-ci` can ever see a conclusion |
| knope labels the PR `no-changeset` | No run created | `BUG-15`'s corrective re-run never fires (`BUG-34`) |

`BUG-28` was fixed by removing the dependency — `publish-release.yml` now owns build and upload
rather than relying on a tag trigger. That worked because there was a dependency to remove. The other
two are not removable the same way: a PR's checks and a label's effect on them are GitHub's own
mechanisms, not ours to restructure.

The result is two manual steps per release, one of them undocumented and not discoverable from the
failure it causes. `v0.2.1` needed both.

## Proposal

Mint an installation access token from a GitHub App and use it for the steps whose events must
trigger workflows:

```yaml
- uses: actions/create-github-app-token@<pinned sha>
  id: app-token
  with:
    app-id: ${{ vars.RELEASE_APP_ID }}
    private-key: ${{ secrets.RELEASE_APP_PRIVATE_KEY }}

- uses: actions/checkout@<pinned sha>
  with:
    token: ${{ steps.app-token.outputs.token }}
```

and pass the same token to knope's `GITHUB_TOKEN` env for `CreatePullRequest` and the `gh pr edit`
label step.

Scope the App to this repository, with the minimum permissions the flow needs: `contents: write`
(branch and tag), `pull_requests: write` (open and label). The token is installation-scoped and
expires in an hour, unlike a PAT.

Expected effects, each checkable on the next release:

- The release PR's `ci` runs without approval.
- The `labeled` event fires, so `BUG-15`'s fix does its job and the changeset gate skips correctly.
- A tag push would trigger a tag workflow again — **not relied on**. `BUG-28`'s fix stands on its own
  and should not be reverted to depend on a trigger a token choice could quietly remove again.

## Why not the alternatives

| Option | Why not |
|---|---|
| Keep the manual steps | Two per release, one undocumented; `BUG-34` shows the failure message actively misleads (it says to add a label that is already there). |
| A classic PAT | Same trigger behaviour, but long-lived, tied to a person, and broader than this repo. An App token is scoped and expires. |
| Re-enable recursion by other means | The guard is protecting something real; the goal is to act as a distinct identity, not to defeat it. |
| Disable the changeset gate for the release branch | Removes a check to avoid fixing why it misfires, and `ADR-29` is explicit that the opt-out must be visible rather than silent. |

## Open questions

- An App is new external surface and a private key to hold. `ADR-45` already rejected a hosted knope
  App on "no evidenced need" grounds — there is evidenced need now, but this should be weighed
  against simply accepting two manual steps per release for a project releasing this rarely.
- Does the App identity need to be excluded anywhere — a `CODEOWNERS` rule, a branch protection
  bypass — or does it inherit sensibly?
- `main` currently has **no branch protection at all**, so nothing gates any merge. If protection is
  added later with `ci` required, the release PR becomes unmergeable without this change. Worth
  deciding the two together.
- Does this make `verify-ci` redundant? Its purpose is checking that `ci` concluded on the released
  tree; if `ci` reliably runs and could be made a required check, the gate may belong to branch
  protection rather than to a workflow step.

## References

- BUG-28 -- the silent instance; fixed by removing the dependency rather than the cause.
- BUG-34 -- the labelling instance, and the undocumented manual step it creates.
- BUG-33 -- adjacent, and not fixed by this: `no_release` is knope's behaviour, not a trigger problem.
- BUG-15 -- the fix this proposal would make effective.
- ADR-45 -- the two-step release flow, and its prior rejection of a hosted knope App.
- SPEC-20 -- the CI/CD reference this changes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial proposal, `Status: Draft`. **Why:** three defects traced to one cause across two releases, two of which cannot be fixed by removing a dependency the way `BUG-28` was. Filed rather than built: it adds an external identity and a private key, which `ADR-45` previously declined on a weaker version of this evidence. | **structural** |
