---
Stable-Id: 01M21FPQMYDX23D28Q48V8D815
Status: Planned
Phase: '0'
Track: schema-governance
Blocked-on: —
---
# 89 — urzua new can collide with a number already claimed by an open, unmerged PR

## What

`next_display_number` only scans filenames in the current local working tree. It has no way to know
another branch -- open as a PR on the remote, not yet merged -- already claimed the next number for
that type. Two real fixes worth weighing, not yet chosen:

- **Local-only, cheap**: scan every local branch's tree for the type's filenames, not just the
  current one -- catches collisions with branches already fetched, not ones only known to GitHub.
- **Remote-aware, needs network**: query open PRs via `gh`/the GitHub API for files under the
  type's directory -- catches everything, but adds a real dependency this tool has avoided so far
  (`urzua` shells out to `git`, never to `gh`, today).

## Why

Found live: filed `RFC-22` on one branch (a `blocker` record type discussion), then on a *second*,
unrelated branch based on the same pre-merge `main`, `urzua new rfc` happily generated `RFC-22`
again for a completely different subject -- caught only because the collision was noticed by eye
before either branch was pushed. Silent if it hadn't been: two real RFCs with the same number,
discovered only once both merged and `filename.title-consistency`/numbering rules had no way to
flag it either, since each branch in isolation looks perfectly valid.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial milestone. **Why:** found live, a real RFC-22/RFC-22 collision across two branches based on the same pre-merge main, caught only by chance before either was pushed. | **structural** |
