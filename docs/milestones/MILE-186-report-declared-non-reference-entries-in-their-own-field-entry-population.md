---
Stable-Id: 01M6MF5W5K7R6PVMV1B71HRG3C
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/report.rs#PopulationUnit@sha256:bc49a15d892e012516c577454358857f9958649311ce6dc6985a6b31ab504ace, spec:docs/specs/SPEC-2-urzua-check.md, test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#declared_entries_are_their_own_field_entry_population_observed_failing@sha256:28a8964a7715de7fc27082b877ec12304f6f1894ffbaa68c5bdc39617295e6e3, test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#a_run_over_no_declarations_reports_zero_eligible_entries_observed_failing@sha256:d990a12be4b7cf05d05413af70c4be3fa75a42fe77b292604dcf126399b4a2d3, test:rust/crates/urzua-core/src/rules/citation.rs#each_declared_entry_is_one_field_entry_candidate_observed_failing@sha256:3178e8f583d47851c54296acaf99c6862ed91525f1860e4ec8cee14bb946c511, test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#the_field_entry_population_is_exercised_on_this_repositorys_own_corpus_observed_failing@sha256:9f8b3656a9bc24a12f967d3df52c9564798bded038206700b57279918c7ec960
---
# 186 — Report declared non-reference entries in their own field-entry population

## What

`citation.resolution`'s check of `non_reference_field` entries (a declared id the record never cites, and a malformed entry) is reported under its own population, with a new `PopulationUnit` for a field entry, so a run over no declarations is distinguishable from a clean one.

## Why

`ADR-75` clause 3 says that check has its own population (unit: field entry). `BUG-190` built the check inside `citation.resolution`, whose one population counts citations, because a rule has one population and a second one changes the output contract (a new `PopulationUnit`, and either a second execution or a separate rule). That is a maintainer decision, so it was deferred.

## Done means

- A `PopulationUnit` for a field entry exists and is documented in `SPEC-2`, with the output-contract change recorded. — evidence: code:rust/crates/urzua-core/src/report.rs#PopulationUnit@sha256:bc49a15d892e012516c577454358857f9958649311ce6dc6985a6b31ab504ace, spec:docs/specs/SPEC-2-urzua-check.md
- A run with `non_reference_field` set reports a field-entry population counting each declared entry. — evidence: test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#declared_entries_are_their_own_field_entry_population_observed_failing@sha256:28a8964a7715de7fc27082b877ec12304f6f1894ffbaa68c5bdc39617295e6e3
- A run over no declarations reports zero eligible entries. — evidence: test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#a_run_over_no_declarations_reports_zero_eligible_entries_observed_failing@sha256:d990a12be4b7cf05d05413af70c4be3fa75a42fe77b292604dcf126399b4a2d3
- The uncited-declaration and malformed-entry findings are counted in the field-entry population. — evidence: test:rust/crates/urzua-core/src/rules/citation.rs#each_declared_entry_is_one_field_entry_candidate_observed_failing@sha256:3178e8f583d47851c54296acaf99c6862ed91525f1860e4ec8cee14bb946c511
- `config.scope-matches-nothing` does not read the field-entry population as a misconfiguration. — evidence: test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#declared_entries_are_their_own_field_entry_population_observed_failing@sha256:28a8964a7715de7fc27082b877ec12304f6f1894ffbaa68c5bdc39617295e6e3
- It is exercised on `ADR-18` and `RFC-5` in this repository's own corpus. — evidence: test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs#the_field_entry_population_is_exercised_on_this_repositorys_own_corpus_observed_failing@sha256:9f8b3656a9bc24a12f967d3df52c9564798bded038206700b57279918c7ec960

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** the field-entry population `ADR-75` clause 3 asks for was deferred by the `BUG-190` build. | **substantive** |
> | 2026-11-05 | Delivered and set `Done`: `PopulationUnit::FieldEntry` (`field-entry`) is documented in `SPEC-2` with the output-contract change; `citation.resolution` with `non_reference_field` set reports a second `rules_executed` entry counting each declared entry (zero eligible over no declarations); the uncited and malformed findings are counted there and `config.scope-matches-nothing` does not read it as a misconfiguration; it is exercised on `ADR-18` and `RFC-5` in this repository's own corpus (two eligible entries, both examined). **Why:** each Done-means item is met and tested end to end; the maintainer reviewing the pull request is the human decision. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
