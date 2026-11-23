---
Stable-Id: 01M85B81TNHQR5HF5105MDW5Q4
Status: Planned
Phase: '1'
Track: release-process
Blocked-on: null
---
# 364 — The release runbook says how to recover each half-finished publish

## What

Document, in `docs/specs/SPEC-20-ci-cd-pipeline.md` and in the release pages under `docs/reference`, what to do
for each point at which `publish-release.yml` can stop after the release pull request has merged, and say which
re-run to use. Where a point has no safe re-run, the milestone names the missing mechanism and files it.

## Why

Found while reviewing the release flow: SPEC-20 covers one case, "if `verify` fails after the release pull
request has merged ... re-run the failed `publish release` workflow run; do not tag by hand" (line 217). It
does not cover a failure after `knope release` has run. At that point the tag and the GitHub Release already
exist, so "Re-run all jobs" runs `release` again against a tag that exists and cannot be expected to recover,
while "Re-run failed jobs" re-runs only `build` or `upload` and can. `upload` passes `--clobber` for that
reason (`SPEC-20` revision of 2026-11-07), but nothing says to choose it. No page under `docs/reference`
mentions recovery. Which button to press, and what to do if the `release` job itself failed after pushing the
tag, is currently knowledge in a maintainer's head. What `knope release` does on a repeat run against an
existing tag was not tested here and has to be established first.

## Done means

- SPEC-20 lists each stopping point (`verify`, `release` before and after the tag, `build`, `upload`) with the re-run that recovers it, each checked against a real or planted failure.
- It states which of them leave a public release with no assets (see `BUG-220`) and what to do to the release meanwhile.
- A case with no safe recovery is filed as its own record, not described away.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
