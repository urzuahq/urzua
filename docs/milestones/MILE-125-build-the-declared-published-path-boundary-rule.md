---
Stable-Id: 01M6CXZXG5N2XF4J9AZ8TFGBXG
Status: Done
Phase: '0'
Track: schema-governance
Implements: RFC-11, ADR-80
Realized-by: code:rust/crates/urzua-core/src/rules/boundary.rs, code:rust/crates/urzua-core/src/rules/back_pointer.rs, code:rust/crates/urzua-core/src/config.rs, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-h/expectations/a-published-boundarys-severity-sets-its-findings-level.yaml
---
# 125 — Build the declared published-path boundary rule

## What

Build the declared published-path boundary rule `RFC-11` decided: config declares a set of paths that
publish beyond the repository, and a record identifier cited from inside one is a violation. Per
`RFC-11` section 1, a boundary is a declared fact, not a rule; section 2, severity is part of the
declaration; section 3, a violation and evidence are separate questions about the same text.

## Why

`RFC-11` is Accepted and its boundary mechanism is unbuilt. A repository that publishes a subset of
its tree keeps internal references out of it by convention only, which is the failure `RFC-11` exists
to remove, and an adopter replacing a hand-written linter outright (`MILE-102`) has to be able to
delete whatever script enforces the boundary today.

## Done means

A declared set of published paths (`published_paths`) makes `boundary.published-path-reference` report a record identifier cited from inside one, the rule's level is the boundary's severity, and the parts of `RFC-11` sections 1 to 3 still unbuilt are built or decided against: a boundary's `kind` and `reason`, glob matching, and the ignore-marker and fixture-exclusion questions; a fixture covers each, and urzua's own `.urzua/config.yaml` declares its published paths and `check` reports none unwaived.

## References

- `RFC-11` -- declared boundaries; sections 1 to 3.
- `ADR-80` -- the decision this builds: kinds, reasons, severities, exceptions.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `RFC-11` (Accepted) decided declared published-path boundaries and nothing builds them. | **substantive** |
> | 2026-11-03 | Built `boundary.published-path-reference` (`published_paths` option; the rule's level is the boundary's severity) in `rules/boundary.rs`. **Why:** `RFC-11` sections 1 to 3 decided it. Section 1's `kind` and `reason`, glob matching, and the ignore-marker and fixture-exclusion questions are not built; `Status` is unchanged. | **substantive** |
> | 2026-11-04 | Evidence so far, no `Realized-by` yet. **Why:** `boundary.published-path-reference` (`rules/boundary.rs`, `published_boundary_integration.rs`, merged in #158) is built; `RFC-11` section 1's `kind` and `reason`, glob matching and the ignore-marker and fixture-exclusion questions are not. `Status` stays Planned. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Realized-by` added and `Status` Planned -> Done. **Why:** built as `ADR-80` decides (#326). A boundary is `paths` (prefixes or `MILE-158` globs), a `kind` (`published`, `vendored`, `generated`), a required `reason` and a `severity` that is the level of its findings; `published_paths` is read as a published boundary; a record id inside a published boundary and a back-pointer inside a generated one are reported; the evidence collector skips published paths and keeps its narrow matcher; `exclude` globs and a declared `ignore_marker` are the exceptions, and one that matches nothing is reported. A fixture covers each (`published_boundary_integration.rs`, `corpus-h`), `SPEC-3` and `SPEC-2` document it, and urzua's own `.urzua/config.yaml` declares the gate action as published (its fixtures excluded) and the two generated directory indexes, with `check` at zero unwaived findings. | **structural** |
