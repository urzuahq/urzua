---
Stable-Id: 01M6SW2V5SF3KYDQM48EGVXQTN
Status: Proposed
Date: 2026-11-07
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 95 — The release pull request is opened by a GitHub App token, and main is protected with ci required

## Context

`prepare-release.yml` opens the `release` pull request. Until `ADR-45`'s flow met a protected `main`, it did
so with `GITHUB_TOKEN`, and GitHub starts no workflow run from a `GITHUB_TOKEN` event. The release PR's own
`ci` run therefore never executed (`RFC-30`, `BUG-34`). `ADR-47` made that harmless while `main` had no
protection, by moving the pre-publish gate into `publish-release.yml`'s `verify` job. It also rejected the
App token of `RFC-30`, and in its Reversibility section left the door open: adopting `RFC-30` later "remains
open if a future need -- a required status check on `main`, say -- makes the approval genuinely blocking
rather than merely tedious".

That need arrived. The maintainer decided on 2026-11-05 ("Protect + app token") to protect `main` with a
ruleset that requires the `ci` check, and to open the release PR with a GitHub App installation token so that
`ci` runs on it. A required check cannot be satisfied by a pull request on which the workflow never runs, so
the two halves stand or fall together; `RFC-30` already said to decide them together.

The workflow side is built: `prepare-release.yml` mints a token with `actions/create-github-app-token` from
the repository secrets `RELEASE_APP_ID` and `RELEASE_APP_PRIVATE_KEY`, fails the job when either is unset,
and uses the token for the checkout, the `release` branch push, knope's `CreatePullRequest` and the
`no-changeset` label. That is the proposal of `RFC-30`, which is still `Draft`. The rule in `AGENTS.md` and
`docs/rfc/README.md` is that when part of an RFC is built, that decision gets its own ADR. No ADR recorded
this one, so `SPEC-20` and the workflow comments described a pipeline that no longer existed. This record
closes that gap. It does not accept `RFC-30`, which stays `Draft`.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep `GITHUB_TOKEN` and leave `main` unprotected (`ADR-47` as it stood) | No new identity or secret | Nothing gates any merge; `ci` cannot be required, because it never runs on the release PR |
| B. Protect `main` with `ci` required, and open the release PR with a GitHub App token (chosen) | `ci` runs on the release PR like any other, with no approval click; the required check is honest; the token is short-lived and scoped to this repository | A new external identity and a private key to hold; the token runs repository code in the release job, so its scope matters |
| C. Protect `main`, open the release PR with a classic PAT | Same trigger behaviour as an App token | Long-lived, tied to a person, broader than this repository (`RFC-30`) |
| D. Protect `main` but put the release bot on the ruleset bypass list | No App | The release PR merges with no check, which defeats the point of requiring one; a bypass actor is itself a standing exception |
| E. Protect `main`, approve the held `ci` run on each release PR by hand | No new identity | Restores the manual step `ADR-47` deleted, and a held run is reported as a failure |

## Decision

*In the context of a `main` that the maintainer wants protected by a required `ci` check, facing a release
pull request on which `GITHUB_TOKEN` can never start that check, we decided that `prepare-release.yml` opens
the release PR with a short-lived installation token minted from a GitHub App, requested with only
`contents: write` and `pull-requests: write` and used for the checkout, the push, the pull request and its
label, and that `main` is protected by a ruleset that requires `ci` with an empty bypass list, to make the
required check run on the release PR honestly, accepting an external identity, a private key to hold, and a
token that runs repository code inside the release job.*

1. **Identity and scope.** One GitHub App owned by the maintainer, installed on this repository only, with the
   repository permissions Contents and Pull requests (read and write) and no webhook. The workflow requests
   an installation token narrowed to those two permissions, not every permission the App holds.
2. **Who holds the key.** The maintainer. The private key lives only in the repository secret
   `RELEASE_APP_PRIVATE_KEY` (the App id in `RELEASE_APP_ID`); the local copy is deleted after it is stored.
   The workflow has no fallback to `GITHUB_TOKEN`: with either secret unset the job fails with a message
   saying so, so the release PR is never quietly opened by an identity whose events start no run.
3. **Rotation.** Not decided here. A rotation is a new key generated in the App settings and stored in the
   secret; no schedule is set. If the maintainer wants one, it is a later record.
4. **The ruleset.** `main` requires a pull request and the `ci` check (the aggregate job of `ci.yml`, GitHub
   Actions source), blocks force pushes and deletions, and has no bypass actor. The release PR is an ordinary
   pull request under it. The ruleset is a repository setting, not a file, so it is applied by the maintainer;
   this record states the intent and does not prove the setting.
5. **What does not change.** `publish-release.yml` still runs `verify` on the merge commit and still tags with
   `GITHUB_TOKEN` (a tag push needs no workflow run to start; `BUG-28`). `ADR-47`'s decision stands, so
   nothing relies on `ci` having run on the release PR. This record uses only the clause of `ADR-47` that
   left adopting `RFC-30` open.

## Reversibility

Moderate. The workflow change reverts in one commit, and deleting the secrets turns the job into a loud
failure rather than a silent downgrade. The App can be uninstalled and its key deleted at any time. What is
sticky is the ruleset: once `ci` is required, the release PR is unmergeable without a run on it, so going
back to `GITHUB_TOKEN` also means removing the requirement or approving a held run on every release.

## Consequences

- The release PR's `ci` run executes without approval and is the same check every other pull request meets.
  The red mark `RFC-30` described stops appearing on routine releases.
- A private key now exists whose compromise lets its holder push to any branch not covered by the ruleset and
  open pull requests as the App. The token's narrowed scope and the empty bypass list bound that; the key is
  still a standing secret, and rotation is unscheduled (clause 3).
- The release job runs repository code (`cargo check`, `cargo run ... harness update`) with the App token in
  `.git/config`. Narrowing the token to two permissions limits what that code can do; it does not make the
  job safe against a malicious change merged to `main`, which review and the ruleset guard.
- Setup lives outside the repository: the App, the two secrets, the ruleset and the permission to create and
  force-push the `release` branch. A missing piece fails visibly (the secrets check) or by blocking a merge,
  never by passing.
- `RFC-30` stays `Draft`; this record and `SPEC-20` carry what was built. Its open questions about a bypass
  actor and about `verify-ci` are answered here (no bypass; `verify` stays, `ADR-47`).
- `SPEC-20` is revised to describe the pipeline as built.

## References

- `RFC-30` -- the proposal this record builds part of; still `Draft`.
- `ADR-45` -- the two-step release flow this opens the pull request for.
- `ADR-47` -- moved the gate to `publish-release.yml` and left adopting `RFC-30` open for a required check.
- `BUG-28` -- why the publish workflow does not depend on a tag trigger.
- `BUG-34` -- the held-run and stale-label failures that follow from `GITHUB_TOKEN` events.
- `SPEC-20` -- the CI/CD pipeline reference, revised alongside this record.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05 ("Protect + app token"): protect `main` with `ci` required and open the release PR with a GitHub App token so `ci` runs on it. The workflow already builds `RFC-30`'s proposal with no ADR, which the RFC rules forbid. The maintainer accepts it in a later change. | **substantive** |
