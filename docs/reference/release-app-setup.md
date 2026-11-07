# Release App and ruleset setup

Maintainer steps for the release pipeline of [ADR-95](../adr/ADR-95-the-release-pull-request-is-opened-by-a-github-app-token-and-main-is-protected-with-ci-required.md)
and [SPEC-20](../specs/SPEC-20-ci-cd-pipeline.md). Nothing here is a file in the repository: the App, the
two secrets and the ruleset are settings only a maintainer can make. `prepare-release.yml` fails with a
pointer to this page until the two secrets are set, on every push to `main`, even when there is nothing to
release.

The secrets are named exactly `RELEASE_APP_CLIENT_ID` and `RELEASE_APP_PRIVATE_KEY`.

## Steps

1. Create a GitHub App owned by you, with no webhook and these repository permissions only: Contents
   (read and write) and Pull requests (read and write).
2. Install the App on `urzuahq/urzua` only.
3. Generate a private key for the App and copy the App's Client ID (not the App ID: the workflow passes it
   as `client-id`).
4. Set two repository secrets: `RELEASE_APP_CLIENT_ID` (the Client ID) and `RELEASE_APP_PRIVATE_KEY` (the
   whole PEM file). Then delete the local copy of the key.
5. Create a ruleset that targets `main` only. It must not cover the `release` branch or `v*` tags. It
   should:
   - require a pull request, with 0 required approvals (with more, a sole maintainer cannot merge);
   - require the status check `ci`, from GitHub Actions;
   - block force pushes and deletions;
   - have an empty bypass list.
6. Confirm that the `release` branch and `v*` tags are unrestricted. A rule covering `release` blocks the
   workflow's force-push of that branch, and a rule covering `v*` blocks the tag push that `knope release`
   makes with `GITHUB_TOKEN` in `publish-release.yml`.
7. Let `prepare-release` open the release pull request on the next push to `main` that carries a changeset,
   and check that `ci` runs on it without approval.

## What goes wrong

- Either secret unset or empty: the "require the release app secrets" step fails, so every push to `main`
  shows a red `prepare release` run. This is deliberate; there is no fallback to `GITHUB_TOKEN`.
- A wrong Client ID or a malformed key: the run fails at the token step instead.
- The ruleset covering `release` or `v*`: the release branch push or the tag push is rejected.
- The ruleset requiring approvals: the release pull request cannot be merged by one maintainer.

## Rotating the key

Generate a new private key in the App settings, store it in `RELEASE_APP_PRIVATE_KEY`, and delete the old
key from the App. No schedule is set (ADR-95, clause 3).
