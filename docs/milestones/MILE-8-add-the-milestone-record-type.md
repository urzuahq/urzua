---
Status: Done
Stable-Id: 01M1Y5JGA8FSBE13XZS36KJR6K
Phase: '0'
Track: roadmap-tracking
Implements: ADR-34
Blocked-on: —
Realized-by: code:.urzua/config.yaml, spec:docs/specs/SPEC-6-the-milestone-record-type.md, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_milestone_must_state_its_done_means_unless_it_is_wontdo@sha256:25ce42b874d80cc7d3a2ab911a79c88c7494b512d80f465ad0b3f621af7e9a16
---
# 8 — Add the milestone record type

## What

A configured record type (Status/Phase/Track/Implements) tracking this project's own backlog, zero urzua-core changes.

## Why

The backlog was accumulating in conversation history and an unstructured document with nowhere durable or checkable to live.

## Done means

- A `milestone` record type is configured in `.urzua/config.yaml` with `Status`, `Phase` and `Track` required and `Implements` a pointer field. — evidence: code:.urzua/config.yaml
- `SPEC-6` documents it. — evidence: spec:docs/specs/SPEC-6-the-milestone-record-type.md
- `urzua check` examines every `docs/milestones/` record under it. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_milestone_must_state_its_done_means_unless_it_is_wontdo@sha256:25ce42b874d80cc7d3a2ab911a79c88c7494b512d80f465ad0b3f621af7e9a16
- Adding it needed no change to urzua-core. — waived: process-outcome -- a property of the change that added the type (commit 784fa758: a config declaration, a template and a spec, with two unrelated core defects fixed beside it), which no file can evidence now

> **Revision log**
>
> | Date | Change | Class |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> |---|---|---|
| 2026-09-07 | Initial record: already complete at creation time. | structural |
