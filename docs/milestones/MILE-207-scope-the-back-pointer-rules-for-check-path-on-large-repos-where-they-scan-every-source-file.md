---
Stable-Id: 01M6SNCTYDQX8F9XMB862VHV42
Status: Planned
Phase: '1'
Track: scale-and-analytics
Blocked-on: null
---
# 207 — Scope the back-pointer rules for check PATH on large repos, where they scan every source file

## What

Scope the source-comment back-pointer rules (`embodiment.back-pointer-resolution` and
`embodiment.back-pointer-decision-accepted`) for a path-scoped run, so `urzua check <path>` on a large
repository does not read every source file under the declared `back_pointer_paths` to report on a few
records. The mechanism is for the milestone's design to settle; this record only names the cost to remove.

## Why

On a large repository `check <path>` takes about 4.6 s, because these rules scan every source file whatever
the path asked about: the scan is the cost and the records in scope are not. Other corpus-wide work is
already scoped to the records a path or `--base` run reports on (`SPEC-2`: drift and claim ages are computed
for those records only), and a path-scoped run is the one the editor hook and the post-command check make
after every edit, so the scan is paid on every edit. The maintainer placed this in Phase 1 on 2026-11-06:
it does not block the Phase 0 exit.

## Done means

- A path-scoped `urzua check <path>` on a repository with the back-pointer rules enabled reads only the source files it needs for the records in scope, and reports the same findings for those records as a whole-corpus `check` does.
- A test with a source file that cites a record outside the scope shows it is not read, and one that cites a record inside the scope shows it is.
- A whole-corpus `check` and a `--base` run are unchanged: the same findings, the same populations.
- The rule population still says what was examined, so a scoped run is distinguishable from one that read nothing (`ADR-55`).
- The saving is measured on a repository of the size that costs about 4.6 s today, and the before and after times are recorded in this record's revision log.
- `SPEC-2` states the scoped read, with a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial milestone. **Why:** found while scoping `check <path>` on a large repository, where the back-pointer rules scan every source file and the run takes about 4.6 s; the maintainer placed it in Phase 1 on 2026-11-06. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
