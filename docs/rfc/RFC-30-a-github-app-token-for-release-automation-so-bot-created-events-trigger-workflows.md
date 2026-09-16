---
Stable-Id: 01M2MMFZ5XEV4R5BX3DFGZ5PZM
Status: Draft
Date: 2026-09-16
Author: beauwilliams
---
# 30 — A GitHub App token for release automation, so bot-created events trigger workflows

## Summary

Every automated step in the release flow acts as `GITHUB_TOKEN`, and GitHub deliberately does not
trigger workflows from `GITHUB_TOKEN` events. Three separate defects follow from that one fact.
Propose minting a short-lived installation token from a GitHub App for the push, PR-creation, and
labelling steps, so those events behave like a real actor's.

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
