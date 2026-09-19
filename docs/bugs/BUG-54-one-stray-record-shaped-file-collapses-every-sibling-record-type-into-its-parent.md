---
Stable-Id: 01M2W327VQ8DVP2E0EH788TC69
Status: Fixed
Found-in: 'A cumulative code review of v0.3.0..release, run before publishing 0.4.0 -- reproduced against the built binary'
Regression-test: 'rust/crates/urzua-cli/src/commands/init.rs::a_container_directory_does_not_absorb_the_types_beneath_it_observed_failing -- adr and rfc survive, the container is not proposed, and BUG-43''s archive/ case still folds'
---
# 54 — One stray record-shaped file collapses every sibling record type into its parent

## What is wrong

`BUG-43` changed the containment filter so the **outer** directory wins and nested records fold into
it. Correct for `doc/adr` against `doc/adr/archive`. Wrong when the outer directory is only
incidentally a parent.

Reproduced against the built binary:

```text
docs/adr/0001-a.md   docs/adr/0002-b.md
docs/rfc/0001-r.md
docs/README.md
docs/0099-index.md          <- one stray record-shaped file

$ urzua init --dry-run
name='doc'  dir='docs'  records=4
```

`adr` and `rfc` collapse into a single `doc` type, and `check` against the written config then examines
`docs/README.md`, **which is not a record**.

The old code skipped files sitting directly under the scan root, and `BUG-36` removed that guard when
it replaced the hardcoded `docs/`.

## Why it matters

The proposed config is silently wrong in a way an adopter is unlikely to notice: every rule still
runs, every record is still examined, and the type names are plausible. A corpus governed as one type
loses every per-type distinction -- different `required_fields`, different `spec` pointers, different
`pointer_fields` -- with no error anywhere.

## Fix

Fold only when the outer directory is genuinely the type's home, not when it merely contains one.
Candidates, unchosen:

- Refuse to fold across a directory that holds **more than one** proposed type beneath it -- `docs/`
  holding `adr` and `rfc` is a container, `doc/adr` holding `archive` is not.
- Require the outer directory's own record count to dominate what it would absorb.

The first reads closer to what an adopter means, and is checkable without a threshold.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed, blocking the 0.4.0 release. **Why:** `BUG-43`'s outer-wins fold is right for an `archive/` subdirectory and wrong for a container directory, and `BUG-36` had already removed the guard that skipped files directly under the scan root. Two fixes from this release cycle combining into a third defect, none of them covered by a test. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** the fold now skips a directory that contains more than one proposed type -- a container, not a home. The first attempt failed because the enclosing-directory lookup included the directory itself, so it never saw `docs/` and the check never fired. Verified: `adr` and `rfc` survive as distinct types, `docs/README.md` is no longer ingested, and `BUG-43`'s `archive/` case still folds. Observed failing. | **substantive** |
> | 2026-09-19 | Regression-test status corrected: `a_container_directory_does_not_absorb_the_types_beneath_it_observed_failing` exists and asserts the `archive/` fold alongside it. Also extended -- review found that proposing the container itself as a type made discovery count every nested record twice (`files_examined: 8` for four records), so a directory with more than one type beneath it is now dropped rather than merely not folded into. | **substantive** |
> | 2026-09-19 | `Regression-test` now names the test that exists. **Why:** the field still read *"not yet written"* after the test was written and observed failing, so this record claimed the work was undone while the work was done. One of eight such records, found by the audit that filed `BUG-57`. | **structural** |
