---
Stable-Id: 01M2VGDWW5DJ1JR7N3JXY8SWQ8
Status: Fixed
Found-in: 'A code review of the unreleased diff since v0.3.0 -- reproduced against a corpus with an `archive/` subdirectory, the shape `adr-tools` repositories commonly have'
Regression-test: 'not yet written -- a corpus with records in both `doc/adr/` and `doc/adr/archive/` must adopt the outer type, and must never silently adopt only the inner one'
---
# 43 — `init` drops the outer record type when a subdirectory also holds records

## What is wrong

`BUG-36` rewrote `detect_record_types` to group by each record's own parent directory, and added a
filter dropping any proposed directory that **contains** another proposed directory -- reasoning that
discovery matches `dir` by path prefix, so an outer directory would claim the inner one's records too.

The filter drops the wrong one. Reproduced against a corpus with three records in `doc/adr/` and one
in `doc/adr/archive/`:

```
proposed:  name='archive'  dir='doc/adr/archive'  records=1
notices:   (none)
```

**Three real records unadopted, the archive governed, and nothing said so.** `check` then reports
success over a corpus it is not examining -- which is precisely the failure `BUG-36`'s own commit
message claimed to be avoiding: *"`check` would then examine zero files and report success"*.

The proposed type name is `archive`, which is also meaningless as a record type.

An `archive/` or `superseded/` subdirectory is ordinary in the corpora this tool is meant to adopt, so
this is not an edge case.

## Why it was missed

The hazard was taken from `MILE-51`'s notes and mitigated from reasoning rather than from an observed
failure. No fixture had a nested record-bearing directory, so the mitigation was never run against the
case it was written for -- and it turned out to resolve it backwards.

## Fix

Prefer the **outer** directory, and report the inner one rather than discarding either silently. An
adopter with an `archive/` subdirectory needs to be told that their corpus has a shape adopt mode
cannot express as one type, not handed a config that quietly governs one record in four.

Whether nested directories should be expressible as one type with a sub-scope, or as two types, is a
real question and belongs with the declared document model (`MILE-98`) rather than in a filter.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** a hazard mitigated from reasoning rather than from an observed failure, and resolved backwards. The filter drops the outer directory, so a corpus with an `archive/` subdirectory adopts one record and leaves three ungoverned, with `check` reporting success over them. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** the containment filter now drops the *inner* directory and folds its records into the nearest enclosing type, so the count an adopter is shown matches what `check` will examine. Verified on the reproduction: a corpus with three records in `doc/adr/` and one in `doc/adr/archive/` proposes `adr` at `doc/adr` with four records, where it previously proposed `archive` with one and left three ungoverned. Observed failing -- reverting the filter makes the test report `left: "doc/adr/archive"`. | **substantive** |
