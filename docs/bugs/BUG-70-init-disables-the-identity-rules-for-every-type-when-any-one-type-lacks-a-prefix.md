---
Stable-Id: 01M2WG82N3NNCD0QBCPWH6QZEB
Status: Fixed
Found-in: "Round 7 of the 0.4.0 review"
Regression-test: "rust/crates/urzua-cli/src/commands/init.rs::one_prefix_less_type_does_not_disarm_the_types_that_have_a_prefix"
---
# 70 — init disables the identity rules for every type when any one type lacks a prefix

## What was wrong

`BUG-61` stopped `init` proposing rules that read a record's identity from its filename when the
corpus carries no type prefix. The guard asks whether *any* proposed type lacks a prefix, and skips
the rule for the whole configuration if one does.

A repository with `docs/adr/ADR-1-x.md` (unanimous `ADR` prefix) and `docs/notes/0001-n.md` (bare
`NNNN-slug`) gets a configuration that declares `prefix: ADR` for `adr` and omits
`filename.title-consistency`, `pointer.resolution`, `pointer.target-status`,
`relation.supersession-reciprocity` and `narrative-field.stale` entirely. `ADR-1-x.md`'s H1 reads
`# 5 - Wrong`, and nothing reports it.

The guard should fire only when *no* proposed type has a prefix. The rules already skip individual
records whose filename yields no identity -- `record_id` returns `None` -- so a mixed corpus costs
nothing but the records that genuinely have no identity to read.

## Why nothing caught it

The test written with the fix, `a_corpus_with_no_prefix_is_not_offered_rules_that_cannot_fire`, uses
a single prefix-less type. With one type, `any` and `all` are the same predicate, so the test passes
against either and cannot fail against the defect.

This is the second test in two rounds that could not fail: `BUG-60`'s first attempt passed against
unfixed code because its fixture omitted `header_shape`.

## References

- `BUG-61`, whose fix introduced this.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
