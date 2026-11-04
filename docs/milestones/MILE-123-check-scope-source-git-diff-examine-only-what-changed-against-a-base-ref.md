---
Stable-Id: 01M6CXZWVJHC0QV4P001H2KS0V
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-cli/src/diff_scope.rs, test:rust/crates/urzua-cli/tests/diff_scope_integration.rs
---
# 123 — check scope source git-diff: examine only what changed against a base ref

## What

Build the `git-diff` value of `scope.source` in `check`'s report: report findings and `files_examined` only for
the records that changed against a base ref, and report the base in the output. It is a reporting
scope, not a reading scope: reference resolution still indexes the full corpus, so an unchanged target
is never reported missing, and rules that need the changed paths themselves (`MILE-124`) receive them
alongside the full index. `SPEC-2` lists it in its scope table as
`git-diff | changed against a base ref | the ref | no -- the mode is not built`.

## Why

A repository of several hundred records should not treat every run as a full sweep in a pull
request gate, and an adopter replacing a linter outright will not accept a slower or noisier one.
`SPEC-2` reserves the value in the report's contract for future use; today `ScopeSource` emits only
`tracked-sweep` and `argv`, and consumers must not branch on a value no run produces. Per no-silent-no-op, the report must say what base it used and what it examined,
not only what it found.

## Done means

`urzua check --base <ref>` reports findings and `files_examined` only for records changed between the merge-base with the ref and the working tree, names the base in the output as `scope.base`, still indexes the full corpus so an unchanged target is never reported missing, matches a full check for waiver changes, comma-joined references, off-directory files and an empty diff, and is refused alongside an untracked named file; `SPEC-2` documents it.

## References

- `SPEC-2` -- the `scope.source` table that records the mode as unbuilt.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `SPEC-2` documents `git-diff` scope as not built; a pull-request gate over a large corpus needs it, so it is now planned. | **substantive** |
> | 2026-11-03 | Built on `urzua check --base <ref>`: merge-base to working tree, reporting-scope only, `scope.base` added, refused alongside an untracked named file. `SPEC-2` revised to match. **Why:** the milestone fixed the semantics (reporting scope, full-corpus index, base reported) but not the flag, the comparison point or the interaction with path arguments; the narrowest reading of each was taken and recorded in `SPEC-2` §Discovery. The `MILE-124` hand-off (changed paths alongside the full index) is not built here: no rule consumes it yet. | **substantive** |
> | 2026-11-04 | Late-review follow-ups to `check --base`: waiver changes, comma-joined references, `type.record-outside-declared-dir` on changed files, symlinked configs and an empty diff with no rule enabled now match a full check; `--base` help reworded. **Why:** each was reproduced against `main` with a failing test; see `SPEC-2`'s revision log. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #164, #154: `urzua check --base <ref>` reports a git-diff scope and names the base. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
