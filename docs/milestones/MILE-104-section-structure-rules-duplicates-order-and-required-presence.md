---
Status: Planned
Stable-Id: 01M2W3XAGYFHAGSQ8NQRP8KBZR
Phase: '0'
Track: section-checks
Implements: SPEC-1
Blocked-on: MILE-98
---
# 104 — Section structure rules: duplicates, order, and required presence

## What

Rules about a record's section *structure*, as opposed to its section *contents*:

- **A section heading appears twice.** Two `## Open questions` in one record is malformed under every
  convention; there is no corpus where it is intended.
- **Sections appear out of a declared order.** No type declares one today, and nothing in the schema
  can express it -- `required_fields` is a set, not a sequence. So this needs a source first: either
  the type's template, whose section order is already the de facto convention every record follows, or
  a new ordered declaration. The template is the better candidate, because it is what an author is
  looking at while writing.
- **A heading level is skipped** -- an `###` with no `##` above it. Defined relative to
  `sections.depth` (`MILE-98`), not absolutely: at `depth: 2` the top level is `##` and a skip is an
  `###` with no `##` above it, while at `depth: 3` the same `###` is an ordinary section. A type that
  declares a depth is stating where its section tree starts, and this rule reads the tree from there
  rather than assuming `##`.

## Not `MILE-4`

`MILE-4` is the policy half: *which* sections a type must have and what must be true of their
contents. This is the structural half: whether the document is well-formed at all, independent of
which sections a type wants.

The distinction matters because they fail differently. A missing required section is a governance
finding -- this corpus expects one and it is absent. A duplicated section is a *broken document*,
and no configuration makes it acceptable.

## Why it is filed

**No rule mentions sections at all.** Measured 2026-09-19: of the twenty-one rules shipping, none
examines section structure, so a record with two `## Open questions` and two `## References` is
well-formed as far as the engine is concerned.

That is not hypothetical. `SPEC-1` shipped in exactly that state during this project's own work --
a narrowing edit inserted new sections without removing the old ones, producing two `## Open questions`
that gave contradictory answers to the same question. `check` was green throughout; review caught it.

Three further record defects in the same period were section-shaped: a revision-log block deleted
along with the section containing it, a revision row appended to a file with no such block, and rows
left dangling after `## References` with no table header. `BUG-50` covers the first two. Nothing
covers the structure they sit in.

## Blocked on `MILE-98`

Sections are not addressable until the document model declares how to find them (`sections.from`, with
`depth` and `items`). This is one more thing waiting on that layer, alongside `MILE-4`, `MILE-5`,
`MILE-6`, `MILE-7`, `MILE-55`, `MILE-97` and six bugs.

## A note on what this does not fix

The mistakes that produced `SPEC-1`'s duplicate sections were *edits that looked applied and were
not*. A rule catches the result; it does not catch the cause. `AGENTS.md` already requires a
planted-violation test observed failing before the fix, and the cause each time was skipping that --
which no rule can enforce.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** none of the twenty-one shipping rules examines section structure, so `SPEC-1` shipped with two `## Open questions` giving contradictory answers and `check` stayed green. Separated from `MILE-4` deliberately: a missing required section is a governance finding, a duplicated one is a broken document, and no configuration makes the second acceptable. | **substantive** |
> | 2026-09-19 | Order and skipped-level cases specified against their sources. **Why:** both were stated absolutely. *Out of a declared order* had no declaration to read -- `required_fields` is a set, not a sequence -- and *a skipped level* assumed `##` is always the top, which `MILE-98`'s `sections.depth` makes configurable. A milestone that cannot say where its rule reads from is a direction, not a deliverable. | **substantive** |
