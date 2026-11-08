---
Status: Done
Stable-Id: 01M1Y5JGA8FSBE13XZS36KJR6K
Phase: '0'
Track: roadmap-tracking
Implements: ADR-34
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089, spec:docs/specs/SPEC-6-the-milestone-record-type.md, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_milestone_must_state_its_done_means_unless_it_is_wontdo@sha256:25ce42b874d80cc7d3a2ab911a79c88c7494b512d80f465ad0b3f621af7e9a16
---
# 8 — Add the milestone record type

## What

A configured record type (Status/Phase/Track/Implements) tracking this project's own backlog, zero urzua-core changes.

## Why

The backlog was accumulating in conversation history and an unstructured document with nowhere durable or checkable to live.

## Done means

- A `milestone` record type is configured in `.urzua/config.yaml` with `Status`, `Phase` and `Track` required and `Implements` a pointer field. — evidence: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089
- `SPEC-6` documents it. — evidence: spec:docs/specs/SPEC-6-the-milestone-record-type.md
- `urzua check` examines every `docs/milestones/` record under it. — evidence: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_milestone_must_state_its_done_means_unless_it_is_wontdo@sha256:25ce42b874d80cc7d3a2ab911a79c88c7494b512d80f465ad0b3f621af7e9a16
- Adding it needed no change to urzua-core. — waived: process-outcome -- a property of the change that added the type (commit 784fa758: a config declaration, a template and a spec, with two unrelated core defects fixed beside it), which no file can evidence now

> **Revision log**
>
> | Date | Change | Class |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> |---|---|---|
| 2026-09-07 | Initial record: already complete at creation time. | structural |
| 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d Reason: Pinned the files as they are now after checking each against the record it evidences: section.rs, document_model.rs, table.rs and config.yaml still hold the decisionless-status exemption (MILE-7), config.yaml and the milestone template still carry the WontDo terminal status (MILE-71), the spec template and config.yaml still make urzua new spec work (MILE-74), field_state.rs, config.yaml, checks.yml and the Makefile still report a returning placeholder (MILE-78), config.yaml still declares the milestone type (MILE-8), table.rs, pointer.rs, commands/rules.rs, generate-rule-table.py and config.yaml still give every rule a configurable level (MILE-80), config.yaml still declares the per-type field sets (MILE-81), config.yaml, pointer.rs and narrative.rs still hold the opt-in narrative-field.stale rule (MILE-83), schema_version.rs, the corpus-a expectation and config.yaml still show the removed shapes refused (MILE-85), and new.rs, new_refs.rs, open_pr.rs, report.rs, config.rs, discovery.rs, check.rs, config.yaml and table.rs still hold the open pull request numbering check (MILE-89). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
| 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
