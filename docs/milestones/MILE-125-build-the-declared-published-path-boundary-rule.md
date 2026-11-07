---
Stable-Id: 01M6CXZXG5N2XF4J9AZ8TFGBXG
Status: Done
Phase: '0'
Track: schema-governance
Implements: RFC-11
Depends-on: ADR-80
Realized-by: code:rust/crates/urzua-core/src/rules/boundary.rs, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_citation_inside_a_published_path_blocks_and_names_file_and_line@sha256:81702a1537c7b61287dcad496287210188e35a3f65f114db6a29582d7fb47198, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#the_same_citation_outside_every_published_path_is_not_reported@sha256:febd5b2bbb7e9d9d02be67de062209037798101edc56ee71c8844c5ab8faf53c, code:rust/crates/urzua-core/src/config.rs#Boundary@sha256:76bdb3e0d67bbb5021357829856c7761c54b8a0b91111279b6b1cb84a516693c, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_boundarys_severity_sets_its_findings_level_and_the_rules_level_only_enables_it@sha256:54f8c7632b589505dfb10de7332f2cdeee43d7e5989553c66a8c86a95030b104, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-h/expectations/a-published-boundarys-severity-sets-its-findings-level.yaml, code:rust/crates/urzua-core/src/rules/back_pointer.rs, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_generated_boundary_reports_a_back_pointer_and_not_a_plain_record_id@sha256:642a1d4f3f618e01abee4ef339415e7d03a09e3dc5a9e6cc43ea1d6a168331e7, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_vendored_boundary_is_declared_and_prohibits_nothing@sha256:0648349c092af4779e9dc356e5f6d7240a20a5007f8992dd812a356222fafcce, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_boundary_without_a_reason_does_not_load@sha256:9c803cecbc5aea957a680c20ec2c97f31443ed19d5021907c585e16d59d91fc6, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_glob_boundary_and_the_compatible_key_apply_together@sha256:377bcd818cc7fce6a1bded018b70a0459dcc6529927ed0f553713ca6660196d8, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#an_exclude_excuses_a_fixture_directory_and_an_unused_one_is_reported@sha256:38b145a04d1e1ac73dda4b3069f2282cd06bdfc29017d41b4953ae6646529ff8, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#the_declared_marker_excuses_its_line_and_a_stale_marker_is_reported@sha256:03c07ba16ef30cf5773ee50234f77499ca358598f2b3ba1247d4fba56cc5cc41, code:.urzua/config.yaml#rules::boundary.published-path-reference@sha256:4a7309eef8e6d959cb41611eef8b9556bb958b4595520bde1761f9ef3d604ccc, test:.github/workflows/checks.yml
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

- A declared set of published paths (`published_paths`) makes `boundary.published-path-reference` report a record identifier cited from inside one, naming the file and line. — evidence: code:rust/crates/urzua-core/src/rules/boundary.rs, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_citation_inside_a_published_path_blocks_and_names_file_and_line@sha256:81702a1537c7b61287dcad496287210188e35a3f65f114db6a29582d7fb47198, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#the_same_citation_outside_every_published_path_is_not_reported@sha256:febd5b2bbb7e9d9d02be67de062209037798101edc56ee71c8844c5ab8faf53c
- The rule's level is the boundary's severity. — evidence: code:rust/crates/urzua-core/src/config.rs#Boundary@sha256:76bdb3e0d67bbb5021357829856c7761c54b8a0b91111279b6b1cb84a516693c, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_boundarys_severity_sets_its_findings_level_and_the_rules_level_only_enables_it@sha256:54f8c7632b589505dfb10de7332f2cdeee43d7e5989553c66a8c86a95030b104, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-h/expectations/a-published-boundarys-severity-sets-its-findings-level.yaml
- A boundary's `kind` and `reason` are built, with a fixture for each. — evidence: code:rust/crates/urzua-core/src/rules/back_pointer.rs, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_generated_boundary_reports_a_back_pointer_and_not_a_plain_record_id@sha256:642a1d4f3f618e01abee4ef339415e7d03a09e3dc5a9e6cc43ea1d6a168331e7, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_vendored_boundary_is_declared_and_prohibits_nothing@sha256:0648349c092af4779e9dc356e5f6d7240a20a5007f8992dd812a356222fafcce, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_boundary_without_a_reason_does_not_load@sha256:9c803cecbc5aea957a680c20ec2c97f31443ed19d5021907c585e16d59d91fc6
- Glob matching of boundary paths is built, with a fixture. — evidence: test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#a_glob_boundary_and_the_compatible_key_apply_together@sha256:377bcd818cc7fce6a1bded018b70a0459dcc6529927ed0f553713ca6660196d8
- The ignore-marker and fixture-exclusion questions are built, with a fixture for each. — evidence: test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#an_exclude_excuses_a_fixture_directory_and_an_unused_one_is_reported@sha256:38b145a04d1e1ac73dda4b3069f2282cd06bdfc29017d41b4953ae6646529ff8, test:rust/crates/urzua-cli/tests/published_boundary_integration.rs#the_declared_marker_excuses_its_line_and_a_stale_marker_is_reported@sha256:03c07ba16ef30cf5773ee50234f77499ca358598f2b3ba1247d4fba56cc5cc41
- Urzua's own `.urzua/config.yaml` declares its published paths and `check` reports none unwaived. — evidence: code:.urzua/config.yaml#rules::boundary.published-path-reference@sha256:4a7309eef8e6d959cb41611eef8b9556bb958b4595520bde1761f9ef3d604ccc, test:.github/workflows/checks.yml

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
> | 2026-11-05 | `Depends-on: ADR-80` added. **Why:** `ADR-80` is the decision record for building `RFC-11` sections 1 to 3 and names `MILE-125` as the milestone it unblocks; building part of an RFC needs its own accepted ADR. `ADR-80` is Accepted as of 2026-11-05, so this dependency is already met. Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Realized-by` added and `Status` Planned -> Done. **Why:** built as `ADR-80` decides (#326). A boundary is `paths` (prefixes or `MILE-158` globs), a `kind` (`published`, `vendored`, `generated`), a required `reason` and a `severity` that is the level of its findings; `published_paths` is read as a published boundary; a record id inside a published boundary and a back-pointer inside a generated one are reported; the evidence collector skips published paths and keeps its narrow matcher; `exclude` globs and a declared `ignore_marker` are the exceptions, and one that matches nothing is reported. A fixture covers each (`published_boundary_integration.rs`, `corpus-h`), `SPEC-3` and `SPEC-2` document it, and urzua's own `.urzua/config.yaml` declares the gate action as published (its fixtures excluded) and the two generated directory indexes, with `check` at zero unwaived findings. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
