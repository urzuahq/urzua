---
Stable-Id: 01M6HZ0KM3DB8TH8K1MMTM5MM5
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Realized-by: code:rust/crates/urzua-core/src/rules/citation.rs, test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs
---
# 136 — A decision ID cited in a record's prose must resolve to a record

## What

A rule over the whole body of a record, not only its declared pointer and narrative fields: every decision ID cited in prose (for example `ADR-12` in a Context paragraph) must resolve to an existing record. An unresolvable citation is an error. Bare-number citations resolve once `MILE-126` settles identity for numeric ids.

## Why

`pointer.resolution` reads only declared header fields, so a prose citation of a record that does not exist, was renumbered or was never created passes every rule today. The adopter's linter checks this across the full text of every decision record, and it is the highest-volume check it runs, so a replacement without it lets dangling citations accumulate unseen. Distinct from `MILE-86`, which asks whether a prose citation should have been a structured pointer; this asks only whether the cited record exists.

## Done means

A planted citation of a nonexistent ID in a record body, in any section, produces an error finding naming the file and line, and removing it clears the finding; a citation inside a fenced code block or an inline code span is handled as the record's config declares; the finding's population reports how many citations were examined, so a run over zero citations is distinguishable from a clean one.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter checks every cited decision ID in the full text of a record, and urzua checks only declared header fields. | **substantive** |
> | 2026-11-04 | Built: `citation.resolution` ships in `rules/citation.rs`, registered in `rules/catalog.rs` and `check`, with `SPEC-2`'s generated rule table updated. Narrowest readings taken: the rule takes no options and reads the prefix of every declared record type; an identifier must stand alone (not preceded by a letter, digit, `-`, `/`, `_` or `.`, not followed by an alphanumeric); the header region and fenced code blocks are skipped while an inline code span is read; numbers compare by value; the population unit is the citation. Bare-number citations remain unbuilt, pending `MILE-126`. **Why:** this milestone left the code-span handling to a declaration that does not exist yet, so the stricter reading (examine it) was taken. `Status` is untouched. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #160: opt-in rule `citation.resolution` resolves decision ids cited in prose and counts the citations examined. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
