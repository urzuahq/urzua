---
Stable-Id: 01M6ME4QBH5GKHM9RP6E3E12KT
Status: Fixed
Found-in: 'review of the declared transitions (MILE-144, ADR-72), 2026-11-04'
Realized-by: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-keeps-its-committed-status.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-may-take-an-allowed-edge.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-reset-to-an-initial-status-is-reported.yaml
---
# 191 — A renamed record is judged as new by status.transition-allowed because the diff reports a rename as a deletion plus an addition

## What was wrong

`urzua check --base` reads a rename as a deletion plus an addition (`changed_paths_since` runs git with
`--no-renames`). `status.transition-allowed` looks the changed record up at its own path in the merge
base, finds nothing, and judges it new. A record renamed (a retitled slug, a moved directory) and moved
from `Accepted` to `Superseded` in the same change is then reported as a new record starting outside
`initial`, instead of being judged against the status it had before the rename. A rename that changes
nothing else is reported the same way when the record's status is not an initial one.

The record's `Stable-Id` is the identity that survives a rename, so the committed side should be
matched by `Stable-Id` across the deleted paths of the diff before the record is called new. This is a
Phase 0 defect: the rule is wrong for a normal editing action, not only an unusual one.

## Why nothing caught it

The acceptance cases write a record in place or add a new file; none renames a tracked record. The
rule's description names the limitation, which documents it without testing it.

## Fix

Fixed 2026-11-05. `committed_statuses` reads the merge base's copy at the record's own path; when there is none, it matches the record by `Stable-Id` against the records the diff deleted (same type) and takes that copy's status. No `Stable-Id`, or no match, still reads as new; several matches count the record unreadable. The base text read added for `BUG-51` (`#279`) is what makes the lookup possible.

## References

- `MILE-144`, `ADR-72`; `status.transition-allowed` in `rust/crates/urzua-core/src/rules/status.rs`;
  `committed_statuses` in `rust/crates/urzua-cli/src/diff_scope.rs`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed, `Open`. **Why:** found while building the declared transitions; the fix (match a renamed record by `Stable-Id`) is deliberately not part of that change. | **substantive** |
> | 2026-11-05 | Fixed. **Why:** `committed_statuses` now matches a record the diff reports as added to a deleted record by `Stable-Id`; three acceptance cases in `corpus-j` (rename with the status unchanged, rename with an allowed edge, rename with a reset to an initial status) failed before and pass after. `SPEC-2` and `SPEC-3` document it. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
