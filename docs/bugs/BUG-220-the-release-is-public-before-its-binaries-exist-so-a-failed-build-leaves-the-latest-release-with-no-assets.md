---
Stable-Id: 01M85B7S3DDGEPC81GNFGS717R
Status: Open
Found-in: 'reviewing the release flow: publish-release.yml creates the release in one job and attaches the archives in a later one'
Realized-by:
---
# 220 — The release is public before its binaries exist, so a failed build leaves the latest release with no assets

## What was wrong

In `.github/workflows/publish-release.yml` the `release` job runs `knope release` (line 77), which tags the
commit and creates the GitHub Release as a published release at once. The `build` job (`needs: release`,
line 96) compiles the two archives and the `upload` job (`needs: [release, build]`, line 149) attaches them.
If either build fails, `upload` never runs and the release stays public with no assets.

A public release with no assets is what `releases/latest` resolves to, and `scripts/install.sh` resolves
the latest release through that redirect (`scripts/install-test.sh`, "no version resolves the latest
release through the redirect"). So every unpinned install fails with a download error until someone repairs
the release by hand. This is the same shape as `BUG-28` (a release that looks published and cannot be
installed). The `upload` job's assertion of two assets runs only after the release is already public, so it
reports the defect and does not prevent it.

## Why nothing caught it

`BUG-28` added an assertion that the finished release carries its archives and `ADR-45`'s amendment recorded
it as the guard. A check that runs after the release is public cannot keep it from being public. No test or
record asks whether the release object may exist before its assets do.

## Options, not decided here

Create the release as a draft and publish it after `upload` has attached and verified both archives. Whether
`knope` 0.23.0's `Release` step can create a draft was not verified; if it cannot, the release is created by
`gh release create --draft` with the changelog section as `--notes-file`, which changes how `ADR-45` says
the notes are produced and so needs a decision. `upload` already passes `--clobber` and the tag is created
before the draft, so the recovery path in `MILE-364` is shortened by a draft, not removed.

## References

- `.github/workflows/publish-release.yml` (lines 77, 96, 149, 171-182), `scripts/install.sh`, `BUG-28`, `ADR-45`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
