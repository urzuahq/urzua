---
Status: Done
Stable-Id: 01M1Y5JF83Z09VVDSV35G4BS2W
Phase: '1'
Track: section-checks
Implements: RFC-17, RFC-18
Blocked-on: —
---
# 6 — Add a doctor check for template/config section agreement

## What

doctor verifies each type's template contains a heading for every section its config declares required.

## Why

Nothing today checks that a template and its config haven't drifted apart -- the same class of silent gap the header_shape/template bug already demonstrated once.

## Done means

- `urzua doctor` reports, per record type, each section the config declares required that the type's template has no heading for. — evidence: code:rust/crates/urzua-cli/src/commands/doctor.rs#template_section_checks@sha256:2f867c4bed614d9a5d246ea4f62400a03a54fc45b9fe937fbbb9db64f1a8dc52, code:rust/crates/urzua-core/src/rules/section.rs#template_missing_sections@sha256:69fa7df7f76adaf1d640b72b4c78b5df881e68c3ff2e6f1a7d582abde8204d56, test:rust/crates/urzua-cli/tests/doctor_template_sections_integration.rs#a_template_missing_a_required_section_is_reported_by_name@sha256:2d772b96ec10f8c3f47363b4d67f8b2397f222a36960c3197086804082783ff3
- A fixture with a template missing one reports it, and a matching template reports clean. — evidence: test:rust/crates/urzua-cli/tests/doctor_template_sections_integration.rs#a_template_missing_a_required_section_is_reported_by_name@sha256:2d772b96ec10f8c3f47363b4d67f8b2397f222a36960c3197086804082783ff3, test:rust/crates/urzua-cli/tests/doctor_template_sections_integration.rs#a_matching_template_reports_clean_and_an_alias_satisfies_a_requirement@sha256:a3a663ce6a2e96aed72237c5f13a4c1fce67d12e5b2f419b80ecf1506923139c
- The check judges presence exactly as `section.required` does: one shared test, with a marker in a fence or the header not counting. — evidence: code:rust/crates/urzua-core/src/rules/section.rs#required_is_met@sha256:f3634fc117a932442b477c04e2f0e84aa6516515b7ded03fa923fbd02f5f9573, test:rust/crates/urzua-cli/tests/doctor_template_sections_integration.rs#doctor_and_section_required_agree_on_a_marker_in_prose_a_fence_and_the_header@sha256:20a3e01662abe796f1f1cc8d54db5008577f554713bdb65d9b0c906fbe078156
- The check runs against urzua's own `.urzua/config.yaml` and `.urzua/templates/` and finds no disagreement. — evidence: test:rust/crates/urzua-cli/tests/doctor_template_sections_integration.rs#this_repositorys_own_templates_agree_with_its_own_configuration@sha256:3a2fd7efb395e6d3bcd6918c5c26dec903fb1f338ab1679d97f22b29ab80612a

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Added `RFC-18` to `Implements`. **Why:** RFC-18 (widening `doctor` beyond CI-wiring health) explicitly folds this milestone in, and names a real gap this milestone's own text hadn't caught: checking a template *exists* isn't the same as checking it has a heading for every required section — narrower than "template/config section agreement" actually promises. | **substantive** |
> | 2026-09-17 | Blocked-on recorded against `MILE-4`. **Why:** doctor cannot verify a template carries a heading for every declared section until sections are declared, which is the document model's job. | **substantive** |
> | 2026-09-17 | `Blocked-on` now names the blocking record rather than describing it. **Why:** it held a sentence (*'Milestone: Build the closed section-parser...'*) which is legal -- `Blocked-on` is a declared `narrative_field`, and prose is what those are for, since "blocked on a decision about X" is a real value no pointer expresses. Changed because in *these three* cases the prose described exactly one existing record, so naming it lets `narrative-field.stale` see the dependency and report when the target goes terminal. Not a defect being fixed; a narrative value replaced by a more useful one. | **structural** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | `Blocked-on` now reads `MILE-98`. **Why:** `narrative-field.stale` flagged a blocker that reached `Done`, and the delivery backfill of `MILE-183` set it; the remaining blocker is still open. `Status` unchanged. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared: `MILE-98` is Done (declared `identity`, `fields` and `sections`, including `###` headings read through `sections.depth`). `Status` unchanged. **Why:** `narrative-field.stale` flagged a blocker that reached a terminal status; the dependency no longer holds. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-13 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (3 of 3 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-13 | Delivered: the `template-sections` check of `urzua doctor` (`SPEC-15`); `Done means` items now carry their pinned evidence. A template missing a required section is a `warn`, not an `error`: it is a configuration disagreement a repository can live with until its next record, and an adopter's existing `doctor` exit code does not change; a type with no template is `ok` (nothing to compare) and a type declaring no required section is not listed. **Why:** `MILE-6`, `RFC-18`; the check reads headings as a record's are read (`depth`, aliases, code fences), through the one `required_is_met` test `section.required` uses, with the template's header region skipped and fences ignored, so the two agree on what counts as present. | **substantive** |
