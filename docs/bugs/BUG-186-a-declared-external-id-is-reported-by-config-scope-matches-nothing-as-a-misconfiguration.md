---
Stable-Id: 01M6JS58VTV16JK13MJWQZTPG6
Status: Fixed
Found-in: 'enabling citation.resolution with external_ids in this repository (2026-11-04)'
Regression-test: 'rust/crates/urzua-core/src/rules/citation.rs::declared_external_ids_do_not_make_scope_matches_nothing_fire_observed_failing, rust/crates/urzua-cli/tests/citation_resolution_integration.rs::a_declared_external_id_is_not_a_misconfiguration_end_to_end_observed_failing'
---
# 186 — A declared external id is reported by config.scope-matches-nothing as a misconfiguration

## What was wrong

`citation.resolution` counted a citation of a declared `external_ids` entry (a public standard number sharing a record prefix) as `OutOfScope`, the outcome that means the configuration cannot reach the candidate. `config.scope-matches-nothing` reads any out-of-scope count as a configuration that fails to reach its candidates and warned that the rule "was handed 2 candidate(s) the configuration does not reach". The warning was false: the ids are an input the repository declared on purpose, set aside by design, not a defect in the configuration. Under `fail_on: warning` it failed the build, and the only remedy was a standing waiver.

## Why nothing caught it

The external-id tests asserted that the citation was skipped and counted out of scope, which is the behaviour that caused the warning; none ran `config.scope-matches-nothing` over the rule's execution. The same trap was fixed for `diff.source-requires-record` exclusions (`MILE-158`) and for the `--base` narrowing (`BUG-180`), and each rule's choice of outcome was reviewed on its own.

## References

Fixed in `citation_resolution`: a citation matching `external_ids` is `Absent` (eligible and unexamined, set aside on purpose), the same classification `MILE-158` gave excluded paths, so it stays visible in the population and is not an unreachable candidate. `SPEC-2` and `SPEC-3` are updated.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and fixed. **Why:** the own-repo rollout of `citation.resolution` (`MILE-156`) needed a standing waiver for a declaration working as intended. | **substantive** |
