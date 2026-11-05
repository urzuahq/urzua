---
Stable-Id: 01M6D2KH7Y2XCM5ZSFDH0VDWNQ
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Realized-by: test:rust/crates/urzua-cli/tests/new_integration.rs#new_refuses_a_title_with_no_slug_and_writes_nothing@sha256:58ae8420b3e79615626f409f3820fc5a1fc1b7f8c0344f0a49a8c5d16e0e1b75
---
# 167 — urzua new with a title that slugifies to nothing creates a slug-less filename and the next new reissues the same number

## What was wrong

`urzua new adr ''` (or a whitespace-only title, or `'日本語'`: `slugify` keeps ASCII alphanumerics only)
wrote `docs/adr/ADR-6-.md`. `parse_record_filename` requires a non-empty segment after the number, so
the numbering scan ignored that file and the next `urzua new adr` took 6 again, producing
`ADR-6-title.md`. `identity.collision` then reported `ADR-6` claimed by two records. A whitespace title
a second time hit "already exists -- refusing to overwrite" on `ADR-6-.md`, leaving numbering stuck at 6.

The tool created the state its own check flags. A 300-character title also fails at write with an OS
error instead of a bounded slug (not changed here).

## Why nothing caught it

The same shape as `BUG-58`, `BUG-133` and `BUG-139`: the code that builds a record filename and the code
that recognises one are separate definitions of the filename grammar, and each is blind to what the other
produces. Tests only passed titles that slugify to something.

## Fix

SPEC-12 does not say what an unsluggable title does, so it is an error: `new` returns a could-not-run
report (exit 2) saying the title has no ASCII letters or digits, before anything is written, and the
number is not consumed. The no-title path, which defaults to `Title`, is unchanged. The change is local
to `new.rs`.

## Notes

Unicode-preserving slugs (so a non-ASCII title is usable instead of rejected) would be a change to the
slug grammar shared with `parse_record_filename` and `record_id` -- a decision to make with the
`BUG-133`/`BUG-139` consolidation, not here.

## References

- `BUG-58` -- same symptom (`new` reissues one number) from a different filename the scan missed.
- `BUG-133`, `BUG-139` -- the duplicated filename grammar underneath.
- `SPEC-12` -- `urzua new`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed in the same change. **Why:** found by a full-release code review and reproduced with the release binary on a disposable fixture; the regression test was observed failing before the fix and passing after. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
