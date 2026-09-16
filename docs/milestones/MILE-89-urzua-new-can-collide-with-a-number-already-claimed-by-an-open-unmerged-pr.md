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

## References

- RFC-13 -- "a vacated number is a record, not a gap": the aftermath half of this same failure
  class (what to do once a collision forced a renumber), proposed but not yet decided. This
  milestone is the prevention half; RFC-13's tombstone is what a real collision resolves into once
  it happens anyway.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial milestone. **Why:** found live, a real RFC-22/RFC-22 collision across two branches based on the same pre-merge main, caught only by chance before either was pushed. | **structural** |
> | 2026-09-16 | Recurred, with a wider blast radius than the original finding. Filing one RFC and one bug against `main` with four PRs open, `urzua new` assigned `RFC-26` and `BUG-19` -- both already claimed on unmerged branches (`RFC-26`/`RFC-27` and `BUG-22`-`BUG-25` on two PRs, `BUG-19`-`BUG-21` on a third). Wrong on both record types, in the same session. Caught only by querying the remote branches by hand before writing -- the exact manual ritual this milestone exists to remove, and one an agent has no reason to perform unless it already knows about this failure. **Why:** the original entry was a single two-branch case caught by eye; this is evidence the failure scales with the number of open PRs and is now routine rather than incidental, which bears on how the two candidate fixes above get weighed -- the local-branch scan would have caught none of these, since the claiming branches existed only on the remote. | **substantive** |
