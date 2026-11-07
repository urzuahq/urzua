---
Stable-Id: 01M6HZ0KM3DB8TH8K1MMTM5MM5
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Realized-by: code:rust/crates/urzua-core/src/rules/citation.rs#citation_resolution@sha256:108ba7dfd2178d4788e2803040222d8962f0a622c890f30f97e07f6b36d23c27, test:rust/crates/urzua-core/src/rules/citation.rs#a_citation_of_a_missing_record_is_an_error_naming_file_and_line_observed_failing@sha256:3da6d28f756a9b1af75f3aace17fb210a1d443d55562546bd668c0a86aa7d0ba, test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#a_declared_rule_reports_a_dangling_citation_and_counts_citations@sha256:90c9d4bdaf9c664c039735a1f9c8c2eba5292539ddd10f404d7831d0c859a155, test:rust/crates/urzua-core/src/rules/citation.rs#removing_the_citation_clears_the_finding@sha256:f922bf280a8298f62991a4941f068fa10aca324a5e81ffb3328e30efbd7d91ff, test:rust/crates/urzua-core/src/rules/citation.rs#fenced_code_blocks_are_skipped_and_inline_spans_are_not@sha256:ac1df8a0b742f9d5de0b5a33558c5d06a51cfceab2b45695d0ad321329d966db, test:rust/crates/urzua-core/src/rules/citation.rs#the_population_counts_citations_not_records@sha256:8f789e1b8e85902d08717cbcb1b5981ffffbdaaf787c6cd154d5b441dd9c883f
---
# 136 — A decision ID cited in a record's prose must resolve to a record

## What

A rule over the whole body of a record, not only its declared pointer and narrative fields: every decision ID cited in prose (for example `ADR-12` in a Context paragraph) must resolve to an existing record. An unresolvable citation is an error. Bare-number citations resolve once `MILE-126` settles identity for numeric ids.

## Why

`pointer.resolution` reads only declared header fields, so a prose citation of a record that does not exist, was renumbered or was never created passes every rule today. The adopter's linter checks this across the full text of every decision record, and it is the highest-volume check it runs, so a replacement without it lets dangling citations accumulate unseen. Distinct from `MILE-86`, which asks whether a prose citation should have been a structured pointer; this asks only whether the cited record exists.

## Done means

- A planted citation of a nonexistent ID in a record body, in any section, produces an error finding naming the file and line. — evidence: code:rust/crates/urzua-core/src/rules/citation.rs#citation_resolution@sha256:108ba7dfd2178d4788e2803040222d8962f0a622c890f30f97e07f6b36d23c27, test:rust/crates/urzua-core/src/rules/citation.rs#a_citation_of_a_missing_record_is_an_error_naming_file_and_line_observed_failing@sha256:3da6d28f756a9b1af75f3aace17fb210a1d443d55562546bd668c0a86aa7d0ba, test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#a_declared_rule_reports_a_dangling_citation_and_counts_citations@sha256:90c9d4bdaf9c664c039735a1f9c8c2eba5292539ddd10f404d7831d0c859a155
- Removing the planted citation clears the finding. — evidence: test:rust/crates/urzua-core/src/rules/citation.rs#removing_the_citation_clears_the_finding@sha256:f922bf280a8298f62991a4941f068fa10aca324a5e81ffb3328e30efbd7d91ff
- A citation inside a fenced code block or an inline code span is handled as the record's config declares. — evidence: test:rust/crates/urzua-core/src/rules/citation.rs#fenced_code_blocks_are_skipped_and_inline_spans_are_not@sha256:ac1df8a0b742f9d5de0b5a33558c5d06a51cfceab2b45695d0ad321329d966db
- The finding's population reports how many citations were examined, so a run over zero citations is distinguishable from a clean one. — evidence: test:rust/crates/urzua-core/src/rules/citation.rs#the_population_counts_citations_not_records@sha256:8f789e1b8e85902d08717cbcb1b5981ffffbdaaf787c6cd154d5b441dd9c883f, test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#a_declared_rule_reports_a_dangling_citation_and_counts_citations@sha256:90c9d4bdaf9c664c039735a1f9c8c2eba5292539ddd10f404d7831d0c859a155

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter checks every cited decision ID in the full text of a record, and urzua checks only declared header fields. | **substantive** |
> | 2026-11-04 | Built: `citation.resolution` ships in `rules/citation.rs`, registered in `rules/catalog.rs` and `check`, with `SPEC-2`'s generated rule table updated. Narrowest readings taken: the rule takes no options and reads the prefix of every declared record type; an identifier must stand alone (not preceded by a letter, digit, `-`, `/`, `_` or `.`, not followed by an alphanumeric); the header region and fenced code blocks are skipped while an inline code span is read; numbers compare by value; the population unit is the citation. Bare-number citations remain unbuilt, pending `MILE-126`. **Why:** this milestone left the code-span handling to a declaration that does not exist yet, so the stricter reading (examine it) was taken. `Status` is untouched. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #160: opt-in rule `citation.resolution` resolves decision ids cited in prose and counts the citations examined. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
