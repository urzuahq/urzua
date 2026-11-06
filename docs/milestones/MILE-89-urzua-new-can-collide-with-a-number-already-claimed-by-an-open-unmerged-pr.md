---
Stable-Id: 01M21FPQMYDX23D28Q48V8D815
Status: Planned
Phase: '0'
Track: schema-governance
Implements: ADR-93
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

## Done means

Matching `ADR-93`:

- `urzua new` assigns no number already claimed on any local branch or remote-tracking ref (built in #135,
  tested in `new_integration.rs`).
- With `new.check_open_prs: true`, `urzua new` fetches pull-request head refs with plain git into
  `refs/urzua/pr/*` (patterns from `new.pull_ref_patterns`, default `refs/pull/*/head`; no `gh`, no GraphQL),
  skips every number held there or on any fetched branch, and reports `pull_refs` with the skipped numbers and
  the refs that held them. A fixture with a number claimed only on a pull ref shows it skipped; with the option
  off no remote is touched.
- Concurrent `new` runs are serialized by a lock under the git common directory: two runs started together
  take two different numbers (a test), a stale lock whose pid is gone is removed, and a held lock times out
  with its path named.
- Offline is never a failure: with no remote, an unreachable one or a timeout, `new` exits 0, numbers from the
  local refs it has, and adds a `notices` entry saying a number claimed on an unfetched pull request will not
  be seen. Each case is a test.
- With the rule configured, `identity.open-pr-collision` under `--base` reports an error for an added record
  whose type and number a pull ref holds with a different `Stable-Id`, and not for a record whose own
  `Stable-Id` is on the ref; without `--base` it says so; with no pull refs fetched it notices and counts zero
  refs. `check` itself never fetches. Tests cover each.
- This repository sets `new.check_open_prs: true` and enables the rule, and `urzua new` run here skips a number
  planted on a pull ref; `urzua check` and `check --base origin/main` show 0 unwaived findings.
- `SPEC-12`, `SPEC-3`, `SPEC-2` and the rule table describe the option, the lock, the report and the rule;
  `make records` and `make rule-table-check` pass.

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
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Evidence so far, no `Realized-by` yet. **Why:** `urzua new` skips numbers claimed on every local and remote-tracking ref (`new_record.rs`, `new_integration.rs`, merged in #135); a number claimed on a branch never fetched is still invisible and the remote-aware alternative the Done means weighs was not built. `Status` stays Planned. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-06 | Re-phased from Phase 1 to Phase 0; `Implements` `ADR-93`; Done means replaced to match it. Built so far is unchanged: the local-ref scan. **Why:** maintainer decision of 2026-11-06 moving this milestone to Phase 0 and deciding the remote-aware path, plain git fetch of pull-request head refs, opt in, with a lock, an offline fallback and an opt-in `check` rule (`ADR-93`); the earlier open choice between a local scan and a `gh` query is closed, and the stated-limit alternative in the old Done means is dropped. | **substantive** |
