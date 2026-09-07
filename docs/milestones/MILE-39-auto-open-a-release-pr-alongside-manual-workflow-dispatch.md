# 39 — Auto-open a release PR alongside manual workflow_dispatch

> Status: Blocked
> Stable-Id: 01M1YKX8C584N9ESVE5181FJJE
> Phase: 1
> Track: release-process
> Implements: ADR-29

## What

`prepare-release.yml` currently only supports a manual `workflow_dispatch` that pushes the version
bump and compiled `CHANGELOG.md` directly to `main` (ADR-29's decided mechanism). Add a second path:
on push to `main` when unreleased changesets exist, automatically open (or update) a release PR
carrying that same staged version bump and compiled changelog, so a maintainer reviews and merges a
normal PR instead of having to remember to trigger the dispatch. Keep `workflow_dispatch` working
alongside it. Requires amending ADR-29's decision (currently states "a direct, reviewed push, never
an unattended action on every merge to main") -- opening a PR is not itself cutting a release
(merging still is), but the mechanism described there needs updating, transparently, not silently.

## Why

13 changesets have accumulated since v0.1.0 with no release cut (MILE-40) -- partly because cutting
one requires a maintainer to remember `workflow_dispatch` exists and go trigger it by hand. A PR
that opens itself is a standing, visible reminder instead of a step nobody's watching for.

## Blocked on

MILE-38 (staleness detection for code comments citing an amended record) -- deliberately sequenced
first so that amending ADR-29 here, and updating `prepare-release.yml`'s own `(ADR-29)` citation to
match, becomes the first real test of whether MILE-38's detector fires on genuine content rather
than a synthetic fixture.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
