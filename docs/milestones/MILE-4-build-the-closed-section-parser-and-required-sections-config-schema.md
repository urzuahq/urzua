---
Status: Done
Stable-Id: 01M1Y5JE63FXMT7PV7QJQN7BXV
Phase: '0'
Track: section-checks
Implements: RFC-17
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/section.rs@sha256:f6257018129e88d01d74dfca6d94f3cca4b6de22e0a68296abab2a825610891c, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#declared_rules_report_each_failure_and_count_slots@sha256:fd72db5c46d6b283f04a776dfb01e5fec3d7b8e2e28a7a6dcf2fac68e22c5376, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#a_type_without_the_declaration_has_nothing_eligible@sha256:25a7482f887a7ab0c1603aca9d05f8fbe7c33d7c76801ad6b47e3760a35b5000, test:rust/crates/urzua-core/src/rules/section.rs#a_missing_required_section_is_a_finding_naming_it_and_every_slot_is_counted@sha256:52411e0615975ed35f5423006c1701a5e3e06d5fbba4b9c63262a9ca921b48ce, code:.urzua/config.yaml#record_types::adr@sha256:861c1c8f1abacb992617f578b52768ac75d56b381bfab56adfafe54fef5355cf, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#the_required_sections_and_the_case_variant_key_are_enforced@sha256:00cf2d2ed89e21983a60f1899a6ca36601122276d8982c775364de939a9b7468
---
# 4 — Require sections per type, as a rule over the declared document model

## What

A rule that a record type's declared sections are present, and that each carries the content shape
its type expects.

**Locating sections is no longer part of this.** That is `MILE-98`'s `sections.from`, with `depth` and
`items`. This milestone is the policy half only: *which* sections a type must have, and *what* must be
true of their contents.

## Why

check validates header fields today but nothing validates what's inside a required section's prose -- SPEC-2 names this rule category and it has never been built.

## The incident this milestone would have caught (`BUG-50`, 2026-09-17)

Narrowing `SPEC-1` deleted its "Monorepo layout" section and took the revision-log block with it:

```text
SPEC-1 revision rows before:  22
SPEC-1 revision rows after:    4   (marker gone; the four dangled after ## References)
```

`revision-log.change-class-required` is declared **`error`** in this repository. It stayed green,
because the rule decides its own scope by searching the content for `> **Revision log**` -- so a
record that loses one line leaves the rule's scope silently. The loss was found by review, not by the
tool that exists to find it.

That is this milestone's shape exactly. With `required_sections` declared per type, a `spec` missing
its revision log is a **finding**; a type that declares no such section is out of scope at
`eligible: 0`. Neither requires the rule to infer anything from the corpus, which is what `ADR-53`
says policy must never do.

`BUG-40`'s population work (`PR #89`) makes the absence *countable*. Measured on this repository
2026-09-21: **307 records, 236 examined, 71 with no revision-log marker.** The same predicate
`BUG-50` measured on 2026-09-19 gave 264 / 196 / 68 -- the corpus grew and the absent count tracked
it, which is the arithmetic `BUG-50` says the report never stated.

A count is not a verdict, and nobody watches it. `BUG-50` also notes that **some of those 71 are
legitimate** -- a `waiver` may have nothing to log -- and the engine cannot tell which, because
nothing declares which types carry a revision log. That is this milestone.

## Evidence from MILE-51 (2026-09-16)

Run against `npryce/adr-tools`, whose records carry their metadata in sections — `## Status`,
`## Context`, `## Decision`, `## Consequences` — and nothing else. `check` reads none of it: 9 errors
about a missing header, and no rule that looks at a section. This milestone is gap 4 of the five
`MILE-51` names.

One question to settle before building, surfaced by that run: `ADR-8`'s closed-header model reports
*unknown keys*. Applied to sections, that flags every `## Alternatives` or `## Notes` a foreign corpus
carries — flooding the corpora this is for. `ADR-8` already names the phasing (*"not yet enforced as
an error by default -- that phasing is Phase 6 scope"*) and the trigger (*"once there's a real
adopter's config to test that transition against"*). That adopter now exists, so the question is
answerable rather than open.

Also worth narrowing when this is picked up: this milestone's *"composable content shape per
section"* is `MILE-5`'s y-statement work and is not needed to check a Nygard corpus. Presence alone
closes gap 4.

## Measured, 2026-09-19

Records missing at least one section their own type's template declares:

| type | deviating | of |
|---|---|---|
| `adr` | **31** | 54 |
| `spec` | **14** | 20 |
| `bug` | **19** | 56 |
| `rfc` | 7 | 35 |
| `milestone` | 8 | 104 |
| **total** | **79** | **269** |

Nearly a third of the corpus, and `check` is green on all of it -- no rule examines sections at all.

The drift is not historical. `BUG-38`, `BUG-39` and `BUG-40`, filed the same week, are each missing
`What was wrong`, `Why nothing caught it` and `References`, because they were written with invented
headings instead of the template's. `MILE-100` through `MILE-102` are missing `Why`. `RFC-35` was
written by replacing the scaffold `urzua new` produced, deleting four of its six sections and
substituting different ones -- changing the RFC format for this corpus in a single record, with
nothing to notice.

That is the case for this milestone stated as a number rather than an intuition: a type's template
declares what its records look like, and the corpus has been diverging from it a record at a time.

## Done means

- A type declares required sections under `sections.required` and the opt-in rule `section.required` reports a record that lacks one, counting one slot per record and required section. — evidence: code:rust/crates/urzua-core/src/rules/section.rs@sha256:f6257018129e88d01d74dfca6d94f3cca4b6de22e0a68296abab2a825610891c, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#declared_rules_report_each_failure_and_count_slots@sha256:fd72db5c46d6b283f04a776dfb01e5fec3d7b8e2e28a7a6dcf2fac68e22c5376
- A type without the declaration has nothing eligible. — evidence: test:rust/crates/urzua-cli/tests/section_rules_integration.rs#a_type_without_the_declaration_has_nothing_eligible@sha256:25a7482f887a7ab0c1603aca9d05f8fbe7c33d7c76801ad6b47e3760a35b5000
- A fixture plants a missing section and observes the finding. — evidence: test:rust/crates/urzua-cli/tests/section_rules_integration.rs#declared_rules_report_each_failure_and_count_slots@sha256:fd72db5c46d6b283f04a776dfb01e5fec3d7b8e2e28a7a6dcf2fac68e22c5376, test:rust/crates/urzua-core/src/rules/section.rs#a_missing_required_section_is_a_finding_naming_it_and_every_slot_is_counted@sha256:52411e0615975ed35f5423006c1701a5e3e06d5fbba4b9c63262a9ca921b48ce
- The capability is exercised by urzua's own `.urzua/config.yaml` (ADRs require `Context`, bugs their defect section). — evidence: code:.urzua/config.yaml#record_types::adr@sha256:861c1c8f1abacb992617f578b52768ac75d56b381bfab56adfafe54fef5355cf, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#the_required_sections_and_the_case_variant_key_are_enforced@sha256:00cf2d2ed89e21983a60f1899a6ca36601122276d8982c775364de939a9b7468

> **Revision log**` -- so a
record that loses one line leaves the rule's scope silently. The loss was found by review, not by the
tool that exists to find it.

That is this milestone's shape exactly. With `required_sections` declared per type, a `spec` missing
its revision log is a **finding**; a type that declares no such section is out of scope at
`eligible: 0`. Neither requires the rule to infer anything from the corpus, which is what `ADR-53`
says policy must never do.

`BUG-40`'s population work (`PR #89`) makes the absence *countable*. Measured on this repository
2026-09-21: **307 records, 236 examined, 71 with no revision-log marker.** The same predicate
`BUG-50` measured on 2026-09-19 gave 264 / 196 / 68 -- the corpus grew and the absent count tracked
it, which is the arithmetic `BUG-50` says the report never stated.

A count is not a verdict, and nobody watches it. `BUG-50` also notes that **some of those 71 are
legitimate** -- a `waiver` may have nothing to log -- and the engine cannot tell which, because
nothing declares which types carry a revision log. That is this milestone.

## Evidence from MILE-51 (2026-09-16)

Run against `npryce/adr-tools`, whose records carry their metadata in sections — `## Status`,
`## Context`, `## Decision`, `## Consequences` — and nothing else. `check` reads none of it: 9 errors
about a missing header, and no rule that looks at a section. This milestone is gap 4 of the five
`MILE-51` names.

One question to settle before building, surfaced by that run: `ADR-8`'s closed-header model reports
*unknown keys*. Applied to sections, that flags every `## Alternatives` or `## Notes` a foreign corpus
carries — flooding the corpora this is for. `ADR-8` already names the phasing (*"not yet enforced as
an error by default -- that phasing is Phase 6 scope"*) and the trigger (*"once there's a real
adopter's config to test that transition against"*). That adopter now exists, so the question is
answerable rather than open.

Also worth narrowing when this is picked up: this milestone's *"composable content shape per
section"* is `MILE-5`'s y-statement work and is not needed to check a Nygard corpus. Presence alone
closes gap 4.

## Measured, 2026-09-19

Records missing at least one section their own type's template declares:

| type | deviating | of |
|---|---|---|
| `adr` | **31** | 54 |
| `spec` | **14** | 20 |
| `bug` | **19** | 56 |
| `rfc` | 7 | 35 |
| `milestone` | 8 | 104 |
| **total** | **79** | **269** |

Nearly a third of the corpus, and `check` is green on all of it -- no rule examines sections at all.

The drift is not historical. `BUG-38`, `BUG-39` and `BUG-40`, filed the same week, are each missing
`What was wrong`, `Why nothing caught it` and `References`, because they were written with invented
headings instead of the template's. `MILE-100` through `MILE-102` are missing `Why`. `RFC-35` was
written by replacing the scaffold `urzua new` produced, deleting four of its six sections and
substituting different ones -- changing the RFC format for this corpus in a single record, with
nothing to notice.

That is the case for this milestone stated as a number rather than an intuition: a type's template
declares what its records look like, and the corpus has been diverging from it a record at a time.

## Done means

A type declares required sections under `sections.required` and the opt-in rule `section.required` reports a record that lacks one, counting one slot per record and required section; a type without the declaration has nothing eligible; a fixture plants a missing section and observes the finding, and the capability is exercised by urzua's own `.urzua/config.yaml` (ADRs require `Context`, bugs their defect section).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Absorbed into `RFC-33`'s declared document model. **Why:** this was scoped as a standalone section parser plus a `required_sections` schema. Under `ADR-53` those are two different layers: locating sections is `sections.from` in the document model (shape, `RFC-34`), and requiring them is a rule (policy). Building it as one thing would rebuild the conflation `ADR-53` exists to remove. The MADR paper test also already moved the target -- `sections.from: h2` is too flat, and `depth`/`items` are needed. | **substantive** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
> | 2026-09-19 | Re-scoped: the section *parser* moves to `MILE-98`, and this keeps `required_sections` as a rule. **Why:** filed as one deliverable -- a parser plus a schema -- which `ADR-53` splits in two. Locating a `##` block is shape and may ship as a primitive (`RFC-34`); requiring one is governance and must be declared. Marked *absorbed into `RFC-33`* on 2026-09-17, which was half right and left this record a duplicate of one third of `MILE-98`. Narrowed instead of closed, because the policy half is real work that nothing else owns. | **substantive** |
> | 2026-09-19 | Measurement added: 79 of 269 records are missing at least one of their type's template sections. **Why:** the case for this milestone had been an intuition. Counted while restoring `RFC-35`, which had been written by replacing its scaffold rather than filling it in -- the same mechanism, visible in records filed the same week. | **substantive** |
> | 2026-11-02 | Unblocked: `RFC-33` was Accepted; maintainer decision 2026-11-02. `Blocked-on` now reads `MILE-98`; `Status` stays `Planned`. **Why:** `narrative-field.stale` flagged the reference once `RFC-33` reached a terminal status, and the maintainer decided the dependency no longer holds. `MILE-98` is still open and stays named. | **substantive** |
> | 2026-11-04 | Built as the opt-in rule `section.required`, reading `required` under a type's declared `sections` (`SPEC-3`). `Status` stays `Planned`. **Why:** `MILE-98` merged the declared `sections` model and `section::parse_sections` and nothing read them. Matching is by exact `##` heading text, the population is one slot per `(record, required section)` so a missing section is counted rather than dropped, and a type that declares no `required` has nothing eligible. Run read-only against this repository's `adr` records with three sections required, 31 required sections were missing across 65 records. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #90: opt-in rule `section.required` reads the declared `sections.required`; the population counts one slot per record and required section. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared: `MILE-98` is Done (declared `identity`, `fields` and `sections`, including `###` headings read through `sections.depth`). `Status` unchanged. **Why:** `narrative-field.stale` flagged a blocker that reached a terminal status; the dependency no longer holds. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::adr` (lines 70-153) 141999030e15 -> 861c1c8f1aba. Reason: Read the diff of each pinned span since the old pin and the claim still holds. config.yaml#record_types::adr: the type's revision_log was expanded from the inline marker and required pair to also declare classes, class_column, date_column and tool_class (the class vocabulary of ADR-94); this change added Last-audited to the type's known_fields, the rest of the declaration is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/section.rs@sha256:f6257018129e Reason: Pinned the files as they are now after checking each against the record it evidences: schema_version.rs, commands/migrate.rs, table.rs and config.yaml still carry the schema version window and the cross-record rules (MILE-34), AGENTS.md still instructs agents to track work in the record types, verify against real runs, never silently rewrite an Accepted decision and not build speculative capability (MILE-36), prepare-release.yml still triggers only on push to main and opens a release pull request with knope.toml and release-guard.sh (MILE-39), section.rs still implements required sections and the y-statement shape (MILE-4, MILE-5), CHANGELOG.md, rust/Cargo.toml and the release workflows still show releases past v0.1.0 consuming changesets (MILE-40), report.rs, action.rs, table.rs and generate-rule-table.py still give every finding a suggested action (MILE-44), report.rs, config.rs and runner.rs still carry the info level (MILE-46), document_model.rs still reads a second corpus's declared model (MILE-51), and the Phase A rule modules, config.yaml and checks.yml still hold the header, pointer, field, filename and supersession rules (MILE-56). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
